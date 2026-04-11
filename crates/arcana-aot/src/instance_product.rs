use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::artifact::AotShackleDeclArtifact;
use crate::native_abi::{
    NativeBindingCallback, NativeBindingImport, parse_native_binding_param,
    parse_native_binding_return_type,
};
use arcana_cabi::{
    ArcanaCabiBindingLayout, ArcanaCabiBindingParam, ArcanaCabiBindingType, ArcanaCabiProductRole,
};
use fs2::FileExt;
use sha2::{Digest, Sha256};

pub const ARCANA_NATIVE_PRODUCT_TEMP_PROBES_ENV: &str = "ARCANA_NATIVE_PRODUCT_TEMP_PROBES";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AotInstanceProductSpec {
    pub package_id: String,
    pub package_name: String,
    pub product_name: String,
    pub role: ArcanaCabiProductRole,
    pub contract_id: String,
    pub output_file_name: String,
    pub package_image_text: Option<String>,
    pub binding_imports: Vec<NativeBindingImport>,
    pub binding_callbacks: Vec<NativeBindingCallback>,
    pub binding_layouts: Vec<ArcanaCabiBindingLayout>,
    pub binding_shackle_decls: Vec<AotShackleDeclArtifact>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AotCompiledInstanceProduct {
    pub output_path: PathBuf,
}

pub fn compile_instance_product(
    spec: &AotInstanceProductSpec,
    project_dir: &Path,
    artifact_dir: &Path,
    cargo_target_dir: &Path,
) -> Result<AotCompiledInstanceProduct, String> {
    if !matches!(
        spec.role,
        ArcanaCabiProductRole::Child
            | ArcanaCabiProductRole::Plugin
            | ArcanaCabiProductRole::Binding
    ) {
        native_product_probe(
            "compile_rejected_role",
            format!(
                "package={} product={} role={}",
                spec.package_name,
                spec.product_name,
                spec.role.as_str()
            ),
        );
        return Err(format!(
            "generic native instance products support only `child`, `plugin`, and `binding` roles (found `{}` for `{}:{}`)",
            spec.role.as_str(),
            spec.package_name,
            spec.product_name
        ));
    }

    native_product_probe(
        "compile_start",
        format!(
            "package={} product={} role={} contract={} project_dir={} artifact_dir={} cargo_target_dir={}",
            spec.package_name,
            spec.product_name,
            spec.role.as_str(),
            spec.contract_id,
            project_dir.display(),
            artifact_dir.display(),
            cargo_target_dir.display()
        ),
    );
    let cargo_toml = render_instance_product_cargo_toml(spec)?;
    let lib_rs = render_instance_product_lib_rs(spec)?;
    let cargo_output_name = instance_product_cargo_output_name(spec);
    let cargo_output_path = cargo_target_dir
        .join("debug")
        .join(cargo_output_file_name(spec, &cargo_output_name)?);
    let output_path = artifact_dir.join("debug").join(&spec.output_file_name);
    let fingerprint = instance_product_inputs_fingerprint(spec, &cargo_toml, &lib_rs)?;

    fs::create_dir_all(artifact_dir).map_err(|e| {
        format!(
            "failed to create native product artifact directory `{}`: {e}",
            artifact_dir.display()
        )
    })?;
    if output_path.is_file()
        && read_inputs_stamp(&instance_product_inputs_stamp_path(artifact_dir))
            .is_some_and(|existing| existing == fingerprint)
    {
        native_product_probe(
            "compile_cache_hit",
            format!(
                "package={} product={} output={}",
                spec.package_name,
                spec.product_name,
                output_path.display()
            ),
        );
        return Ok(AotCompiledInstanceProduct { output_path });
    }

    write_instance_product_project(project_dir, &cargo_toml, &lib_rs)?;

    let manifest_path = project_dir.join("Cargo.toml");
    let _build_lock = acquire_cargo_target_lock(cargo_target_dir)?;
    let status = Command::new("cargo")
        .arg("build")
        .arg("--message-format")
        .arg("short")
        .arg("--manifest-path")
        .arg(&manifest_path)
        .arg("--target-dir")
        .arg(cargo_target_dir)
        .status()
        .map_err(|e| {
            format!(
                "failed to build native product `{}` from `{}`: {e}",
                spec.product_name,
                manifest_path.display()
            )
        })?;
    if !status.success() {
        native_product_probe(
            "compile_failed",
            format!(
                "package={} product={} manifest={} status={status}",
                spec.package_name,
                spec.product_name,
                manifest_path.display()
            ),
        );
        return Err(format!(
            "native product build failed for `{}` from `{}` with status {status}",
            spec.product_name,
            manifest_path.display()
        ));
    }

    if !cargo_output_path.is_file() {
        native_product_probe(
            "compile_missing_output",
            format!(
                "package={} product={} expected_output={}",
                spec.package_name,
                spec.product_name,
                cargo_output_path.display()
            ),
        );
        return Err(format!(
            "generated native product `{}` on `{}` did not produce `{}` under `{}`",
            spec.product_name,
            spec.package_name,
            cargo_output_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("<unknown>"),
            cargo_target_dir.join("debug").display()
        ));
    }
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "failed to create staged native product output directory `{}`: {e}",
                parent.display()
            )
        })?;
    }
    fs::copy(&cargo_output_path, &output_path).map_err(|e| {
        format!(
            "failed to stage generated native product `{}` from `{}` to `{}`: {e}",
            spec.product_name,
            cargo_output_path.display(),
            output_path.display()
        )
    })?;
    write_inputs_stamp(
        &instance_product_inputs_stamp_path(artifact_dir),
        &fingerprint,
    )?;

    native_product_probe(
        "compile_success",
        format!(
            "package={} product={} output={}",
            spec.package_name,
            spec.product_name,
            output_path.display()
        ),
    );
    Ok(AotCompiledInstanceProduct { output_path })
}

fn write_instance_product_project(
    project_dir: &Path,
    cargo_toml: &str,
    lib_rs: &str,
) -> Result<(), String> {
    fs::create_dir_all(project_dir.join("src")).map_err(|e| {
        format!(
            "failed to create generated native product project `{}`: {e}",
            project_dir.display()
        )
    })?;
    write_file_if_changed(&project_dir.join("Cargo.toml"), cargo_toml)?;
    write_file_if_changed(&project_dir.join("src").join("lib.rs"), lib_rs)?;
    Ok(())
}

fn write_file_if_changed(path: &Path, content: &str) -> Result<(), String> {
    if fs::read_to_string(path)
        .ok()
        .is_some_and(|existing| existing == content)
    {
        return Ok(());
    }
    fs::write(path, content).map_err(|e| format!("failed to write `{}`: {e}", path.display()))
}

fn instance_product_inputs_stamp_path(target_dir: &Path) -> PathBuf {
    target_dir.join(".arcana-instance-product.inputs")
}

fn read_inputs_stamp(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok()
}

fn write_inputs_stamp(path: &Path, fingerprint: &str) -> Result<(), String> {
    fs::write(path, fingerprint).map_err(|e| {
        format!(
            "failed to write native product inputs stamp `{}`: {e}",
            path.display()
        )
    })
}

fn acquire_cargo_target_lock(target_dir: &Path) -> Result<std::fs::File, String> {
    fs::create_dir_all(target_dir).map_err(|e| {
        format!(
            "failed to create shared native cargo target directory `{}`: {e}",
            target_dir.display()
        )
    })?;
    let lock_path = target_dir.join(".arcana-cargo-build.lock");
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(|e| {
            format!(
                "failed to open shared native cargo lock `{}`: {e}",
                lock_path.display()
            )
        })?;
    file.lock_exclusive().map_err(|e| {
        format!(
            "failed to lock shared native cargo target directory `{}`: {e}",
            target_dir.display()
        )
    })?;
    Ok(file)
}

fn instance_product_inputs_fingerprint(
    spec: &AotInstanceProductSpec,
    cargo_toml: &str,
    lib_rs: &str,
) -> Result<String, String> {
    let repo_root = repo_root();
    let mut hasher = Sha256::new();
    hasher.update(b"arcana_instance_product_inputs_v1\n");
    hasher.update(format!("package={}\n", spec.package_name).as_bytes());
    hasher.update(format!("product={}\n", spec.product_name).as_bytes());
    hasher.update(format!("role={}\n", spec.role.as_str()).as_bytes());
    hasher.update(format!("contract={}\n", spec.contract_id).as_bytes());
    hasher.update(format!("output={}\n", spec.output_file_name).as_bytes());
    hasher.update(cargo_toml.as_bytes());
    hasher.update(b"\n--lib-rs--\n");
    hasher.update(lib_rs.as_bytes());
    fingerprint_path_contents(&repo_root.join("Cargo.toml"), &mut hasher)?;
    fingerprint_path_contents(&repo_root.join("Cargo.lock"), &mut hasher)?;
    fingerprint_tree_contents(&repo_root.join("crates").join("arcana-cabi"), &mut hasher)?;
    if matches!(spec.role, ArcanaCabiProductRole::Child) {
        fingerprint_tree_contents(
            &repo_root.join("crates").join("arcana-runtime"),
            &mut hasher,
        )?;
    }
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn fingerprint_tree_contents(path: &Path, hasher: &mut Sha256) -> Result<(), String> {
    if !path.exists() {
        hasher.update(format!("missing:{}\n", path.display()).as_bytes());
        return Ok(());
    }
    let mut entries = fs::read_dir(path)
        .map_err(|e| {
            format!(
                "failed to read `{}` for native product fingerprinting: {e}",
                path.display()
            )
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| {
            format!(
                "failed to enumerate `{}` for native product fingerprinting: {e}",
                path.display()
            )
        })?;
    entries.sort_by_key(|entry| entry.path());
    for entry in entries {
        let entry_path = entry.path();
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();
        if name == "target" || name == ".git" {
            continue;
        }
        let metadata = entry.metadata().map_err(|e| {
            format!(
                "failed to read metadata for `{}`: {e}",
                entry_path.display()
            )
        })?;
        if metadata.is_dir() {
            hasher.update(format!("dir:{}\n", entry_path.display()).as_bytes());
            fingerprint_tree_contents(&entry_path, hasher)?;
        } else if metadata.is_file() {
            fingerprint_path_contents(&entry_path, hasher)?;
        }
    }
    Ok(())
}

fn fingerprint_path_contents(path: &Path, hasher: &mut Sha256) -> Result<(), String> {
    let bytes = fs::read(path)
        .map_err(|e| format!("failed to read `{}` for hashing: {e}", path.display()))?;
    hasher.update(format!("file:{}:{}\n", path.display(), bytes.len()).as_bytes());
    hasher.update(&bytes);
    Ok(())
}

fn render_instance_product_cargo_toml(spec: &AotInstanceProductSpec) -> Result<String, String> {
    let repo_root = repo_root();
    let cabi_dependency = repo_root.join("crates").join("arcana-cabi");
    let runtime_dependency = repo_root.join("crates").join("arcana-runtime");
    let cargo_output_name = instance_product_cargo_output_name(spec);
    let mut out = format!(
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
        ),
        escape_toml(&format!(
            "arcana_native_product_{}_{}",
            sanitize_identifier(&spec.package_id),
            sanitize_identifier(&spec.product_name)
        )),
        escape_toml(&cargo_output_name),
        escape_toml(&cabi_dependency.display().to_string()),
    );
    if matches!(spec.role, ArcanaCabiProductRole::Child) {
        out.push_str(&format!(
            "arcana_runtime = {{ package = \"arcana-runtime\", path = \"{}\" }}\n",
            escape_toml(&runtime_dependency.display().to_string())
        ));
    }
    out.push_str("\n[workspace]\n");
    Ok(out)
}

fn render_instance_product_lib_rs(spec: &AotInstanceProductSpec) -> Result<String, String> {
    match spec.role {
        ArcanaCabiProductRole::Child => Ok(render_child_instance_product_lib_rs(spec)),
        ArcanaCabiProductRole::Plugin => Ok(render_plugin_instance_product_lib_rs(spec)),
        ArcanaCabiProductRole::Export => unreachable!("instance products reject export role"),
        ArcanaCabiProductRole::Binding => render_binding_instance_product_lib_rs(spec),
    }
}

fn render_common_instance_preamble(spec: &AotInstanceProductSpec) -> String {
    let package_name = format!("{}\0", spec.package_name);
    let product_name = format!("{}\0", spec.product_name);
    let role = format!("{}\0", spec.role.as_str());
    let contract = format!("{}\0", spec.contract_id);
    format!(
        concat!(
            "use std::cell::RefCell;\n",
            "use std::ffi::{{c_char, c_void}};\n",
            "use std::ptr;\n\n",
            "use arcana_cabi::{{\n",
            "    ARCANA_CABI_CONTRACT_VERSION_V1,\n",
            "    ARCANA_CABI_GET_PRODUCT_API_V1_SYMBOL,\n",
            "    ArcanaCabiCreateInstanceFn,\n",
            "    ArcanaCabiDestroyInstanceFn,\n",
            "    ArcanaCabiLastErrorAllocFn,\n",
            "    ArcanaCabiOwnedBytesFreeFn,\n",
            "    ArcanaCabiOwnedStrFreeFn,\n",
            "    ArcanaCabiProductApiV1,\n",
            "}};\n\n",
            "thread_local! {{\n",
            "    static LAST_ERROR: RefCell<Vec<u8>> = const {{ RefCell::new(Vec::new()) }};\n",
            "}}\n\n",
            "static PACKAGE_NAME: &str = {};\n",
            "static PRODUCT_NAME: &str = {};\n",
            "static ROLE_NAME: &str = {};\n",
            "static CONTRACT_ID: &str = {};\n\n",
            "fn set_last_error(err: String) {{\n",
            "    LAST_ERROR.with(|slot| *slot.borrow_mut() = err.into_bytes());\n",
            "}}\n\n",
            "fn allocated_bytes_parts(bytes: Vec<u8>) -> (*mut u8, usize) {{\n",
            "    if bytes.is_empty() {{\n",
            "        return (ptr::null_mut(), 0);\n",
            "    }}\n",
            "    let len = bytes.len();\n",
            "    (Box::into_raw(bytes.into_boxed_slice()) as *mut u8, len)\n",
            "}}\n\n",
            "unsafe extern \"system\" fn last_error_alloc(out_len: *mut usize) -> *mut u8 {{\n",
            "    let bytes = LAST_ERROR.with(|slot| slot.borrow().clone());\n",
            "    let (ptr, len) = allocated_bytes_parts(bytes);\n",
            "    if !out_len.is_null() {{ unsafe {{ *out_len = len; }} }}\n",
            "    ptr\n",
            "}}\n\n",
            "unsafe extern \"system\" fn owned_bytes_free(ptr: *mut u8, len: usize) {{\n",
            "    if ptr.is_null() || len == 0 {{\n",
            "        return;\n",
            "    }}\n",
            "    unsafe {{ drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len))); }}\n",
            "}}\n\n",
            "unsafe extern \"system\" fn owned_str_free(ptr: *mut u8, len: usize) {{\n",
            "    unsafe {{ owned_bytes_free(ptr, len); }}\n",
            "}}\n\n",
        ),
        render_rust_string_literal(&package_name),
        render_rust_string_literal(&product_name),
        render_rust_string_literal(&role),
        render_rust_string_literal(&contract),
    )
}

fn render_unit_instance_helpers() -> &'static str {
    concat!(
        "unsafe extern \"system\" fn create_unit_instance() -> *mut c_void {\n",
        "    Box::into_raw(Box::new(())) as *mut c_void\n",
        "}\n\n",
        "unsafe extern \"system\" fn destroy_unit_instance(instance: *mut c_void) {\n",
        "    if instance.is_null() {\n",
        "        return;\n",
        "    }\n",
        "    unsafe {\n",
        "        drop(Box::from_raw(instance as *mut ()));\n",
        "    }\n",
        "}\n\n",
    )
}

fn render_child_instance_product_lib_rs(spec: &AotInstanceProductSpec) -> String {
    let mut out = render_common_instance_preamble(spec);
    out.push_str(render_unit_instance_helpers());
    out.push_str(
        concat!(
            "use std::ffi::CStr;\n",
            "use arcana_cabi::{ArcanaCabiChildOpsV1, ArcanaCabiInstanceOpsV1};\n",
            "use arcana_runtime::{current_process_runtime_host, execute_entrypoint_routine, parse_runtime_package_image};\n\n",
            "unsafe extern \"system\" fn run_entrypoint(\n",
            "    instance: *mut c_void,\n",
            "    package_image_ptr: *const u8,\n",
            "    package_image_len: usize,\n",
            "    main_routine_key: *const c_char,\n",
            "    out_exit_code: *mut i32,\n",
            ") -> i32 {\n",
            "    let result = (|| {\n",
            "        if instance.is_null() {\n",
            "            return Err(\"child runtime provider instance must not be null\".to_string());\n",
            "        }\n",
            "        if package_image_ptr.is_null() {\n",
            "            return Err(\"child runtime provider received null package image\".to_string());\n",
            "        }\n",
            "        if main_routine_key.is_null() {\n",
            "            return Err(\"child runtime provider received null main routine key\".to_string());\n",
            "        }\n",
            "        if out_exit_code.is_null() {\n",
            "            return Err(\"child runtime provider requires non-null out_exit_code\".to_string());\n",
            "        }\n",
            "        let package_image = unsafe { std::slice::from_raw_parts(package_image_ptr, package_image_len) };\n",
            "        let package_image_text = std::str::from_utf8(package_image)\n",
            "            .map_err(|e| format!(\"child runtime provider package image is not utf8: {e}\"))?;\n",
            "        let routine_key = unsafe { CStr::from_ptr(main_routine_key) }\n",
            "            .to_str()\n",
            "            .map_err(|e| format!(\"child runtime provider main routine key is not utf8: {e}\"))?;\n",
            "        let plan = parse_runtime_package_image(package_image_text)?;\n",
            "        let mut host = current_process_runtime_host()?;\n",
            "        let exit_code = execute_entrypoint_routine(&plan, routine_key, host.as_mut())?;\n",
            "        unsafe { *out_exit_code = exit_code; }\n",
            "        Ok(())\n",
            "    })();\n",
            "    match result {\n",
            "        Ok(()) => 1,\n",
            "        Err(err) => {\n",
            "            set_last_error(err);\n",
            "            0\n",
            "        }\n",
            "    }\n",
            "}\n\n",
            "static CHILD_OPS: ArcanaCabiChildOpsV1 = ArcanaCabiChildOpsV1 {\n",
            "    base: ArcanaCabiInstanceOpsV1 {\n",
            "        ops_size: std::mem::size_of::<ArcanaCabiInstanceOpsV1>(),\n",
            "        create_instance: create_unit_instance as ArcanaCabiCreateInstanceFn,\n",
            "        destroy_instance: destroy_unit_instance as ArcanaCabiDestroyInstanceFn,\n",
            "        reserved0: ptr::null(),\n",
            "        reserved1: ptr::null(),\n",
            "    },\n",
            "    run_entrypoint,\n",
            "    last_error_alloc: last_error_alloc as ArcanaCabiLastErrorAllocFn,\n",
            "    owned_bytes_free: owned_bytes_free as ArcanaCabiOwnedBytesFreeFn,\n",
            "    reserved0: ptr::null(),\n",
            "    reserved1: ptr::null(),\n",
            "};\n\n",
            "static PRODUCT_API: ArcanaCabiProductApiV1 = ArcanaCabiProductApiV1 {\n",
            "    descriptor_size: std::mem::size_of::<ArcanaCabiProductApiV1>(),\n",
            "    package_name: PACKAGE_NAME.as_ptr() as *const c_char,\n",
            "    product_name: PRODUCT_NAME.as_ptr() as *const c_char,\n",
            "    role: ROLE_NAME.as_ptr() as *const c_char,\n",
            "    contract_id: CONTRACT_ID.as_ptr() as *const c_char,\n",
            "    contract_version: ARCANA_CABI_CONTRACT_VERSION_V1,\n",
            "    role_ops: &CHILD_OPS as *const ArcanaCabiChildOpsV1 as *const c_void,\n",
            "    reserved0: ptr::null(),\n",
            "    reserved1: ptr::null(),\n",
            "};\n\n",
            "const _: &str = ARCANA_CABI_GET_PRODUCT_API_V1_SYMBOL;\n",
            "const _: u32 = ARCANA_CABI_CONTRACT_VERSION_V1;\n\n",
            "#[unsafe(no_mangle)]\n",
            "pub extern \"system\" fn arcana_cabi_get_product_api_v1() -> *const ArcanaCabiProductApiV1 {\n",
            "    &PRODUCT_API\n",
            "}\n",
        ),
    );
    out
}

fn render_plugin_instance_product_lib_rs(spec: &AotInstanceProductSpec) -> String {
    let description = format!(
        "{}:{} [{}]",
        spec.package_name, spec.product_name, spec.contract_id
    );
    let mut out = render_common_instance_preamble(spec);
    out.push_str(render_unit_instance_helpers());
    out.push_str(&format!(
        "static PLUGIN_DESCRIPTION: &str = {};\n\n",
        render_rust_string_literal(&description)
    ));
    out.push_str(
        concat!(
            "use arcana_cabi::{ArcanaCabiInstanceOpsV1, ArcanaCabiPluginOpsV1, ArcanaCabiPluginUseInstanceFn};\n\n",
            "unsafe extern \"system\" fn describe_instance(instance: *mut c_void, out_len: *mut usize) -> *mut u8 {\n",
            "    if instance.is_null() {\n",
            "        set_last_error(\"plugin instance must not be null\".to_string());\n",
            "        if !out_len.is_null() { unsafe { *out_len = 0; } }\n",
            "        return ptr::null_mut();\n",
            "    }\n",
            "    let (ptr, len) = allocated_bytes_parts(PLUGIN_DESCRIPTION.as_bytes().to_vec());\n",
            "    if !out_len.is_null() { unsafe { *out_len = len; } }\n",
            "    ptr\n",
            "}\n\n",
            "unsafe extern \"system\" fn use_instance(instance: *mut c_void, request_ptr: *const u8, request_len: usize, out_len: *mut usize) -> *mut u8 {\n",
            "    if instance.is_null() {\n",
            "        set_last_error(\"plugin instance must not be null\".to_string());\n",
            "        if !out_len.is_null() { unsafe { *out_len = 0; } }\n",
            "        return ptr::null_mut();\n",
            "    }\n",
            "    if request_ptr.is_null() && request_len != 0 {\n",
            "        set_last_error(\"plugin use_instance received null request with non-zero length\".to_string());\n",
            "        if !out_len.is_null() { unsafe { *out_len = 0; } }\n",
            "        return ptr::null_mut();\n",
            "    }\n",
            "    let mut response = PLUGIN_DESCRIPTION.as_bytes().to_vec();\n",
            "    if request_len != 0 {\n",
            "        response.push(b'\\n');\n",
            "        response.extend_from_slice(unsafe { std::slice::from_raw_parts(request_ptr, request_len) });\n",
            "    }\n",
            "    let (ptr, len) = allocated_bytes_parts(response);\n",
            "    if !out_len.is_null() { unsafe { *out_len = len; } }\n",
            "    ptr\n",
            "}\n\n",
            "static PLUGIN_OPS: ArcanaCabiPluginOpsV1 = ArcanaCabiPluginOpsV1 {\n",
            "    base: ArcanaCabiInstanceOpsV1 {\n",
            "        ops_size: std::mem::size_of::<ArcanaCabiInstanceOpsV1>(),\n",
            "        create_instance: create_unit_instance as ArcanaCabiCreateInstanceFn,\n",
            "        destroy_instance: destroy_unit_instance as ArcanaCabiDestroyInstanceFn,\n",
            "        reserved0: ptr::null(),\n",
            "        reserved1: ptr::null(),\n",
            "    },\n",
            "    describe_instance,\n",
            "    use_instance: use_instance as ArcanaCabiPluginUseInstanceFn,\n",
            "    last_error_alloc: last_error_alloc as ArcanaCabiLastErrorAllocFn,\n",
            "    owned_bytes_free: owned_bytes_free as ArcanaCabiOwnedBytesFreeFn,\n",
            "    reserved0: ptr::null(),\n",
            "    reserved1: ptr::null(),\n",
            "};\n\n",
            "static PRODUCT_API: ArcanaCabiProductApiV1 = ArcanaCabiProductApiV1 {\n",
            "    descriptor_size: std::mem::size_of::<ArcanaCabiProductApiV1>(),\n",
            "    package_name: PACKAGE_NAME.as_ptr() as *const c_char,\n",
            "    product_name: PRODUCT_NAME.as_ptr() as *const c_char,\n",
            "    role: ROLE_NAME.as_ptr() as *const c_char,\n",
            "    contract_id: CONTRACT_ID.as_ptr() as *const c_char,\n",
            "    contract_version: ARCANA_CABI_CONTRACT_VERSION_V1,\n",
            "    role_ops: &PLUGIN_OPS as *const ArcanaCabiPluginOpsV1 as *const c_void,\n",
            "    reserved0: ptr::null(),\n",
            "    reserved1: ptr::null(),\n",
            "};\n\n",
            "const _: &str = ARCANA_CABI_GET_PRODUCT_API_V1_SYMBOL;\n",
            "const _: u32 = ARCANA_CABI_CONTRACT_VERSION_V1;\n\n",
            "#[unsafe(no_mangle)]\n",
            "pub extern \"system\" fn arcana_cabi_get_product_api_v1() -> *const ArcanaCabiProductApiV1 {\n",
            "    &PRODUCT_API\n",
            "}\n",
        ),
    );
    out
}

fn render_binding_instance_product_lib_rs(spec: &AotInstanceProductSpec) -> Result<String, String> {
    let package_name = format!("{}\0", spec.package_name);
    let product_name = format!("{}\0", spec.product_name);
    let role = format!("{}\0", spec.role.as_str());
    let contract = format!("{}\0", spec.contract_id);
    let binding_impls = spec
        .binding_imports
        .iter()
        .map(|import| lookup_binding_impl_decl(spec, import))
        .collect::<Result<Vec<_>, _>>()?;
    let has_package_state_decl = spec.binding_shackle_decls.iter().any(|decl| {
        matches!(decl.kind.as_str(), "type" | "struct" | "union" | "flags")
            && decl.name == "PackageState"
    });
    let has_state_init = spec.binding_shackle_decls.iter().any(|decl| {
        decl.kind == "fn" && decl.binding.as_deref() == Some("__binding.package_state_init")
    });
    let has_state_drop = spec.binding_shackle_decls.iter().any(|decl| {
        decl.kind == "fn" && decl.binding.as_deref() == Some("__binding.package_state_drop")
    });
    if (has_package_state_decl || has_state_drop) && !has_state_init {
        return Err(format!(
            "binding instance product `{}` on `{}` declares custom package state but is missing `shackle fn ... = __binding.package_state_init`",
            spec.product_name, spec.package_name
        ));
    }

    let mut out = render_generated_binding_preamble(
        &package_name,
        &product_name,
        &role,
        &contract,
        !has_state_init,
        !has_state_drop,
    );
    out.push_str(&render_shackle_support_items(spec)?);
    out.push_str(&render_binding_runtime_support(
        !has_state_init,
        !has_state_drop,
    ));
    out.push_str(&render_binding_metadata(spec));
    out.push_str(&render_binding_import_impls(spec, &binding_impls)?);
    out.push_str(render_generated_binding_descriptor());
    Ok(out)
}

fn render_binding_metadata(spec: &AotInstanceProductSpec) -> String {
    let mut out = String::new();
    for (import_index, import) in spec.binding_imports.iter().enumerate() {
        out.push_str(&format!(
            "static BINDING_IMPORT_{import_index}_NAME: &str = {};\n",
            render_rust_string_literal(&format!("{}\0", import.name))
        ));
        out.push_str(&format!(
            "static BINDING_IMPORT_{import_index}_SYMBOL: &str = {};\n",
            render_rust_string_literal(&format!("{}\0", import.symbol_name))
        ));
        out.push_str(&format!(
            "static BINDING_IMPORT_{import_index}_RETURN_TYPE: &str = {};\n",
            render_rust_string_literal(&format!("{}\0", import.return_type.render()))
        ));
        out.push_str(&render_binding_param_array(
            &format!("BINDING_IMPORT_{import_index}"),
            &import.params,
        ));
    }
    if !spec.binding_imports.is_empty() {
        out.push_str("static BINDING_IMPORTS: [ArcanaCabiBindingImportEntryV1; ");
        out.push_str(&spec.binding_imports.len().to_string());
        out.push_str("] = [\n");
        for (import_index, import) in spec.binding_imports.iter().enumerate() {
            let params_expr = if import.params.is_empty() {
                "ptr::null()".to_string()
            } else {
                format!("BINDING_IMPORT_{import_index}_PARAMS.as_ptr()")
            };
            out.push_str(&format!(
                "    ArcanaCabiBindingImportEntryV1 {{ name: BINDING_IMPORT_{import_index}_NAME.as_ptr() as *const c_char, symbol_name: BINDING_IMPORT_{import_index}_SYMBOL.as_ptr() as *const c_char, return_type: BINDING_IMPORT_{import_index}_RETURN_TYPE.as_ptr() as *const c_char, params: {params_expr}, param_count: {} }},\n",
                import.params.len(),
            ));
        }
        out.push_str("];\n\n");
    } else {
        out.push_str("static BINDING_IMPORTS: [ArcanaCabiBindingImportEntryV1; 0] = [];\n\n");
    }

    for (callback_index, callback) in spec.binding_callbacks.iter().enumerate() {
        out.push_str(&format!(
            "static BINDING_CALLBACK_{callback_index}_NAME: &str = {};\n",
            render_rust_string_literal(&format!("{}\0", callback.name))
        ));
        out.push_str(&format!(
            "static BINDING_CALLBACK_{callback_index}_RETURN_TYPE: &str = {};\n",
            render_rust_string_literal(&format!("{}\0", callback.return_type.render()))
        ));
        out.push_str(&render_binding_param_array(
            &format!("BINDING_CALLBACK_{callback_index}"),
            &callback.params,
        ));
    }
    if !spec.binding_callbacks.is_empty() {
        out.push_str("static BINDING_CALLBACKS: [ArcanaCabiBindingCallbackEntryV1; ");
        out.push_str(&spec.binding_callbacks.len().to_string());
        out.push_str("] = [\n");
        for (callback_index, callback) in spec.binding_callbacks.iter().enumerate() {
            let params_expr = if callback.params.is_empty() {
                "ptr::null()".to_string()
            } else {
                format!("BINDING_CALLBACK_{callback_index}_PARAMS.as_ptr()")
            };
            out.push_str(&format!(
                "    ArcanaCabiBindingCallbackEntryV1 {{ name: BINDING_CALLBACK_{callback_index}_NAME.as_ptr() as *const c_char, return_type: BINDING_CALLBACK_{callback_index}_RETURN_TYPE.as_ptr() as *const c_char, params: {params_expr}, param_count: {} }},\n",
                callback.params.len(),
            ));
        }
        out.push_str("];\n\n");
    } else {
        out.push_str("static BINDING_CALLBACKS: [ArcanaCabiBindingCallbackEntryV1; 0] = [];\n\n");
    }
    out.push_str("fn binding_callback_name_is_declared(name: &str) -> bool {\n");
    out.push_str("    match name {\n");
    for callback in &spec.binding_callbacks {
        out.push_str(&format!(
            "        {} => true,\n",
            render_rust_string_literal(&callback.name)
        ));
    }
    out.push_str("        _ => false,\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
    if !spec.binding_layouts.is_empty() {
        for (layout_index, layout) in spec.binding_layouts.iter().enumerate() {
            let detail_json = serde_json::to_string(layout)
                .expect("binding layout metadata should serialize to json");
            out.push_str(&format!(
                "static BINDING_LAYOUT_{layout_index}_ID: &str = {};\n",
                render_rust_string_literal(&format!("{}\0", layout.layout_id))
            ));
            out.push_str(&format!(
                "static BINDING_LAYOUT_{layout_index}_DETAIL_JSON: &str = {};\n",
                render_rust_string_literal(&format!("{detail_json}\0"))
            ));
        }
        out.push_str("static BINDING_LAYOUTS: [arcana_cabi::ArcanaCabiBindingLayoutEntryV1; ");
        out.push_str(&spec.binding_layouts.len().to_string());
        out.push_str("] = [\n");
        for (layout_index, _) in spec.binding_layouts.iter().enumerate() {
            out.push_str(&format!(
                "    arcana_cabi::ArcanaCabiBindingLayoutEntryV1 {{ layout_id: BINDING_LAYOUT_{layout_index}_ID.as_ptr() as *const c_char, detail_json: BINDING_LAYOUT_{layout_index}_DETAIL_JSON.as_ptr() as *const c_char }},\n"
            ));
        }
        out.push_str("];\n\n");
    } else {
        out.push_str(
            "static BINDING_LAYOUTS: [arcana_cabi::ArcanaCabiBindingLayoutEntryV1; 0] = [];\n\n",
        );
    }
    out
}

fn render_binding_param_array(prefix: &str, params: &[ArcanaCabiBindingParam]) -> String {
    let mut out = String::new();
    for (param_index, param) in params.iter().enumerate() {
        out.push_str(&format!(
            "static {prefix}_PARAM_{param_index}_NAME: &str = {};\n",
            render_rust_string_literal(&format!("{}\0", param.name))
        ));
        out.push_str(&format!(
            "static {prefix}_PARAM_{param_index}_SOURCE_MODE: &str = {};\n",
            render_rust_string_literal(&format!("{}\0", param.source_mode.as_str()))
        ));
        out.push_str(&format!(
            "static {prefix}_PARAM_{param_index}_PASS_MODE: &str = {};\n",
            render_rust_string_literal(&format!("{}\0", param.pass_mode.as_str()))
        ));
        out.push_str(&format!(
            "static {prefix}_PARAM_{param_index}_INPUT_TYPE: &str = {};\n",
            render_rust_string_literal(&format!("{}\0", param.input_type.render()))
        ));
        if let Some(write_back_type) = &param.write_back_type {
            out.push_str(&format!(
                "static {prefix}_PARAM_{param_index}_WRITE_BACK_TYPE: &str = {};\n",
                render_rust_string_literal(&format!("{}\0", write_back_type.render()))
            ));
        }
    }
    if !params.is_empty() {
        out.push_str(&format!(
            "static {prefix}_PARAMS: [arcana_cabi::ArcanaCabiExportParamV1; {}] = [\n",
            params.len()
        ));
        for (param_index, param) in params.iter().enumerate() {
            let write_back_expr = if param.write_back_type.is_some() {
                format!("{prefix}_PARAM_{param_index}_WRITE_BACK_TYPE.as_ptr() as *const c_char")
            } else {
                "ptr::null()".to_string()
            };
            out.push_str(&format!(
                "    arcana_cabi::ArcanaCabiExportParamV1 {{ name: {prefix}_PARAM_{param_index}_NAME.as_ptr() as *const c_char, source_mode: {prefix}_PARAM_{param_index}_SOURCE_MODE.as_ptr() as *const c_char, pass_mode: {prefix}_PARAM_{param_index}_PASS_MODE.as_ptr() as *const c_char, input_type: {prefix}_PARAM_{param_index}_INPUT_TYPE.as_ptr() as *const c_char, write_back_type: {write_back_expr} }},\n"
            ));
        }
        out.push_str("];\n\n");
    }
    out
}

fn render_binding_import_impls(
    spec: &AotInstanceProductSpec,
    binding_impls: &[&AotShackleDeclArtifact],
) -> Result<String, String> {
    let mut out = String::new();
    for (index, (import, decl)) in spec.binding_imports.iter().zip(binding_impls).enumerate() {
        out.push_str(&format!(
            "#[allow(unused_variables)]\nfn binding_import_impl_{index}(\n    instance: &mut BindingInstance,\n    args: &[ArcanaCabiBindingValueV1],\n    out_write_backs: &mut [ArcanaCabiBindingValueV1],\n) -> Result<ArcanaCabiBindingValueV1, String> {{\n"
        ));
        out.push_str(&format!(
            "    require_arg_count(args.len(), {}, {:?})?;\n",
            import.params.len(),
            import.name
        ));
        for (param_index, param) in import.params.iter().enumerate() {
            out.push_str(&render_binding_param_decode(
                spec,
                param_index,
                param,
                &decl.params[param_index],
            )?);
        }
        out.push_str(&render_binding_import_impl_body(spec, import, decl)?);
        out.push_str("}\n\n");
        out.push_str(&format!(
            "#[unsafe(export_name = {:?})]\npub unsafe extern \"system\" fn binding_import_stub_{index}(\n    instance: *mut c_void,\n    args: *const ArcanaCabiBindingValueV1,\n    arg_count: usize,\n    out_write_backs: *mut ArcanaCabiBindingValueV1,\n    out_result: *mut ArcanaCabiBindingValueV1,\n) -> i32 {{\n    unsafe {{ run_binding_import({:?}, instance, args, arg_count, out_write_backs, out_result, binding_import_impl_{index}) }}\n}}\n\n",
            import.symbol_name,
            import.name
        ));
    }
    Ok(out)
}

fn render_binding_param_decode(
    spec: &AotInstanceProductSpec,
    index: usize,
    param: &ArcanaCabiBindingParam,
    decl_param: &arcana_ir::IrRoutineParam,
) -> Result<String, String> {
    let target_ty = render_shackle_rust_type(spec, &decl_param.ty);
    let reader = match &param.input_type {
        ArcanaCabiBindingType::Int => "read_int_arg".to_string(),
        ArcanaCabiBindingType::Bool => "read_bool_arg".to_string(),
        ArcanaCabiBindingType::Str => "read_utf8_arg".to_string(),
        ArcanaCabiBindingType::Bytes => "read_bytes_arg".to_string(),
        ArcanaCabiBindingType::I8 => "read_i8_arg".to_string(),
        ArcanaCabiBindingType::U8 => "read_u8_arg".to_string(),
        ArcanaCabiBindingType::I16 => "read_i16_arg".to_string(),
        ArcanaCabiBindingType::U16 => "read_u16_arg".to_string(),
        ArcanaCabiBindingType::I32 => "read_i32_arg".to_string(),
        ArcanaCabiBindingType::U32 => "read_u32_arg".to_string(),
        ArcanaCabiBindingType::I64 => "read_i64_arg".to_string(),
        ArcanaCabiBindingType::U64 => "read_u64_arg".to_string(),
        ArcanaCabiBindingType::ISize => "read_isize_arg".to_string(),
        ArcanaCabiBindingType::USize => "read_usize_arg".to_string(),
        ArcanaCabiBindingType::F32 => "read_f32_arg".to_string(),
        ArcanaCabiBindingType::F64 => "read_f64_arg".to_string(),
        ArcanaCabiBindingType::Named(_) => format!("read_layout_arg::<{target_ty}>"),
        ArcanaCabiBindingType::Unit => "read_unit_arg".to_string(),
    };
    let local_name = sanitize_identifier(&param.name);
    let mut out = format!(
        "    let {local_name} = {reader}(&args[{index}], {:?})?;\n",
        param.name
    );
    if param.write_back_type.is_some() {
        out.push_str(&format!(
            "    let {local_name}_write_back = &mut out_write_backs[{index}];\n"
        ));
    }
    Ok(out)
}

fn render_binding_import_impl_body(
    spec: &AotInstanceProductSpec,
    import: &NativeBindingImport,
    decl: &AotShackleDeclArtifact,
) -> Result<String, String> {
    let mut out = String::new();
    if let Some(module_use_path) = shackle_decl_module_use_path(spec, decl) {
        out.push_str(&format!(
            "    #[allow(unused_imports)]\n    use {module_use_path}::*;\n"
        ));
    }
    match decl.kind.as_str() {
        "fn" => {
            if decl.binding.as_deref() == Some(import.name.as_str()) {
                for line in &decl.body_entries {
                    out.push_str("    ");
                    out.push_str(line);
                    out.push('\n');
                }
            } else {
                let args = render_direct_shackle_import_call_args(spec, import, decl)?;
                let call = format!("{}({args})", decl.name);
                out.push_str(&render_binding_result_expr(
                    import,
                    &call,
                    decl,
                    /*is_statement*/ import.return_type == ArcanaCabiBindingType::Unit,
                )?);
            }
        }
        "import fn" | "import_fn" => {
            if import
                .params
                .iter()
                .any(|param| param.write_back_type.is_some())
            {
                return Err(format!(
                    "exported shackle import fn `{}` cannot satisfy binding import `{}` with edit/write-back params",
                    decl.name, import.name
                ));
            }
            let args = render_direct_shackle_import_call_args(spec, import, decl)?;
            let call = format!("unsafe {{ {}({args}) }}", decl.name);
            out.push_str(&render_binding_result_expr(
                import,
                &call,
                decl,
                /*is_statement*/ import.return_type == ArcanaCabiBindingType::Unit,
            )?);
        }
        "const" => {
            let const_expr = decl.name.as_str();
            out.push_str(&render_binding_result_expr(
                import, const_expr, decl, false,
            )?);
        }
        other => {
            return Err(format!(
                "binding import `{}` resolved to unsupported shackle declaration kind `{other}`",
                import.name
            ));
        }
    }
    Ok(out)
}

fn render_direct_shackle_import_call_args(
    spec: &AotInstanceProductSpec,
    import: &NativeBindingImport,
    decl: &AotShackleDeclArtifact,
) -> Result<String, String> {
    if import.params.len() != decl.params.len() {
        return Err(format!(
            "binding import `{}` arg count does not match shackle import fn `{}`",
            import.name, decl.name
        ));
    }
    import
        .params
        .iter()
        .zip(decl.params.iter())
        .map(|(import_param, decl_param)| {
            let local = sanitize_identifier(&import_param.name);
            let target_ty = render_shackle_rust_type(spec, &decl_param.ty);
            let expr = match &import_param.input_type {
                ArcanaCabiBindingType::Int => {
                    if target_ty == "i64" {
                        local
                    } else {
                        format!("{local} as {target_ty}")
                    }
                }
                ArcanaCabiBindingType::Bool => {
                    if target_ty == "bool" {
                        local
                    } else {
                        format!("{local} as {target_ty}")
                    }
                }
                ArcanaCabiBindingType::I8
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
                | ArcanaCabiBindingType::F64 => {
                    if target_ty == rust_scalar_type_name(&import_param.input_type) {
                        local
                    } else {
                        format!("{local} as {target_ty}")
                    }
                }
                ArcanaCabiBindingType::Named(_)
                | ArcanaCabiBindingType::Str
                | ArcanaCabiBindingType::Bytes
                | ArcanaCabiBindingType::Unit => local,
            };
            Ok(expr)
        })
        .collect::<Result<Vec<_>, String>>()
        .map(|parts| parts.join(", "))
}

fn render_binding_result_expr(
    import: &NativeBindingImport,
    expr: &str,
    _decl: &AotShackleDeclArtifact,
    is_statement: bool,
) -> Result<String, String> {
    let line = match &import.return_type {
        ArcanaCabiBindingType::Int => format!("    Ok(binding_int({expr} as i64))\n"),
        ArcanaCabiBindingType::Bool => {
            format!("    Ok(binding_bool({expr}))\n")
        }
        ArcanaCabiBindingType::Str => {
            format!("    Ok(binding_owned_str({expr}))\n")
        }
        ArcanaCabiBindingType::Bytes => {
            format!("    Ok(binding_owned_bytes({expr}))\n")
        }
        ArcanaCabiBindingType::I8 => format!("    Ok(binding_i8({expr} as i8))\n"),
        ArcanaCabiBindingType::U8 => format!("    Ok(binding_u8({expr} as u8))\n"),
        ArcanaCabiBindingType::I16 => format!("    Ok(binding_i16({expr} as i16))\n"),
        ArcanaCabiBindingType::U16 => format!("    Ok(binding_u16({expr} as u16))\n"),
        ArcanaCabiBindingType::I32 => format!("    Ok(binding_i32({expr} as i32))\n"),
        ArcanaCabiBindingType::U32 => format!("    Ok(binding_u32({expr} as u32))\n"),
        ArcanaCabiBindingType::I64 => format!("    Ok(binding_i64({expr} as i64))\n"),
        ArcanaCabiBindingType::U64 => format!("    Ok(binding_u64({expr} as u64))\n"),
        ArcanaCabiBindingType::ISize => format!("    Ok(binding_isize({expr} as isize))\n"),
        ArcanaCabiBindingType::USize => format!("    Ok(binding_usize({expr} as usize))\n"),
        ArcanaCabiBindingType::F32 => format!("    Ok(binding_f32({expr} as f32))\n"),
        ArcanaCabiBindingType::F64 => format!("    Ok(binding_f64({expr} as f64))\n"),
        ArcanaCabiBindingType::Named(_) => {
            format!("    Ok(binding_layout({expr}))\n")
        }
        ArcanaCabiBindingType::Unit => {
            if is_statement {
                format!("    {expr};\n    Ok(binding_unit())\n")
            } else {
                format!("    let _ = {expr};\n    Ok(binding_unit())\n")
            }
        }
    };
    Ok(line)
}

fn rust_scalar_type_name(ty: &ArcanaCabiBindingType) -> &'static str {
    match ty {
        ArcanaCabiBindingType::Int => "i64",
        ArcanaCabiBindingType::Bool => "bool",
        ArcanaCabiBindingType::I8 => "i8",
        ArcanaCabiBindingType::U8 => "u8",
        ArcanaCabiBindingType::I16 => "i16",
        ArcanaCabiBindingType::U16 => "u16",
        ArcanaCabiBindingType::I32 => "i32",
        ArcanaCabiBindingType::U32 => "u32",
        ArcanaCabiBindingType::I64 => "i64",
        ArcanaCabiBindingType::U64 => "u64",
        ArcanaCabiBindingType::ISize => "isize",
        ArcanaCabiBindingType::USize => "usize",
        ArcanaCabiBindingType::F32 => "f32",
        ArcanaCabiBindingType::F64 => "f64",
        ArcanaCabiBindingType::Str => "alloc::string::String",
        ArcanaCabiBindingType::Bytes => "alloc::vec::Vec<u8>",
        ArcanaCabiBindingType::Named(_) | ArcanaCabiBindingType::Unit => "",
    }
}

fn render_generated_binding_preamble(
    package_name: &str,
    product_name: &str,
    role: &str,
    contract: &str,
    needs_default_state_init: bool,
    needs_default_state_drop: bool,
) -> String {
    let mut out = String::new();
    out.push_str(concat!(
        "#![allow(dead_code, non_camel_case_types, non_snake_case, non_upper_case_globals, unsafe_op_in_unsafe_fn)]\n\n",
        "use std::cell::RefCell;\n",
        "use std::collections::BTreeMap;\n",
        "use std::ffi::{c_char, c_void, CStr};\n",
        "use std::ptr;\n\n",
        "use arcana_cabi::{\n",
        "    ARCANA_CABI_CONTRACT_VERSION_V1,\n",
        "    ARCANA_CABI_GET_PRODUCT_API_V1_SYMBOL,\n",
        "    ArcanaCabiBindingCallbackEntryV1,\n",
        "    ArcanaCabiBindingCallbackFn,\n",
        "    ArcanaCabiBindingImportEntryV1,\n",
        "    ArcanaCabiBindingOpsV1,\n",
        "    ArcanaCabiBindingPayloadV1,\n",
        "    ArcanaCabiBindingRegisterCallbackFn,\n",
        "    ArcanaCabiBindingUnregisterCallbackFn,\n",
        "    ArcanaCabiBindingValueTag,\n",
        "    ArcanaCabiBindingValueV1,\n",
        "    ArcanaCabiCreateInstanceFn,\n",
        "    ArcanaCabiDestroyInstanceFn,\n",
        "    ArcanaCabiInstanceOpsV1,\n",
        "    ArcanaCabiLastErrorAllocFn,\n",
        "    ArcanaCabiOwnedBytesFreeFn,\n",
        "    ArcanaCabiOwnedStrFreeFn,\n",
        "    ArcanaCabiProductApiV1,\n",
        "    free_owned_bytes,\n",
        "    free_owned_str,\n",
        "    into_owned_bytes,\n",
        "    into_owned_str,\n",
        "    release_binding_output_value,\n",
        "};\n\n",
        "thread_local! {\n",
        "    static LAST_ERROR: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };\n",
        "}\n\n",
    ));
    out.push_str(&format!(
        "static PACKAGE_NAME: &str = {};\n",
        render_rust_string_literal(package_name)
    ));
    out.push_str(&format!(
        "static PRODUCT_NAME: &str = {};\n",
        render_rust_string_literal(product_name)
    ));
    out.push_str(&format!(
        "static ROLE_NAME: &str = {};\n",
        render_rust_string_literal(role)
    ));
    out.push_str(&format!(
        "static CONTRACT_ID: &str = {};\n\n",
        render_rust_string_literal(contract)
    ));
    if needs_default_state_init {
        out.push_str("type PackageState = ();\n\n");
        out.push_str(
            "fn package_state_init() -> Result<PackageState, String> {\n    Ok(())\n}\n\n",
        );
    }
    if needs_default_state_drop {
        out.push_str("fn package_state_drop(_state: &mut PackageState) {}\n\n");
    }
    out
}

fn render_binding_runtime_support(
    _needs_default_state_init: bool,
    _needs_default_state_drop: bool,
) -> String {
    concat!(
        "fn set_last_error(err: String) {\n",
        "    LAST_ERROR.with(|slot| *slot.borrow_mut() = err.into_bytes());\n",
        "}\n\n",
        "fn allocated_bytes_parts(bytes: Vec<u8>) -> (*mut u8, usize) {\n",
        "    if bytes.is_empty() {\n",
        "        return (ptr::null_mut(), 0);\n",
        "    }\n",
        "    let len = bytes.len();\n",
        "    (Box::into_raw(bytes.into_boxed_slice()) as *mut u8, len)\n",
        "}\n\n",
        "unsafe extern \"system\" fn binding_last_error_alloc(out_len: *mut usize) -> *mut u8 {\n",
        "    let bytes = LAST_ERROR.with(|slot| slot.borrow().clone());\n",
        "    let (ptr, len) = allocated_bytes_parts(bytes);\n",
        "    if !out_len.is_null() {\n",
        "        unsafe { *out_len = len; }\n",
        "    }\n",
        "    ptr\n",
        "}\n\n",
        "unsafe extern \"system\" fn binding_owned_bytes_free(ptr: *mut u8, len: usize) {\n",
        "    unsafe { free_owned_bytes(ptr, len); }\n",
        "}\n\n",
        "unsafe extern \"system\" fn binding_owned_str_free(ptr: *mut u8, len: usize) {\n",
        "    unsafe { free_owned_str(ptr, len); }\n",
        "}\n\n",
        "#[derive(Clone, Copy, Debug)]\n",
        "struct RegisteredCallback {\n",
        "    callback: ArcanaCabiBindingCallbackFn,\n",
        "    owned_bytes_free: ArcanaCabiOwnedBytesFreeFn,\n",
        "    owned_str_free: ArcanaCabiOwnedStrFreeFn,\n",
        "    user_data: *mut c_void,\n",
        "}\n\n",
        "struct BindingInstance {\n",
        "    callbacks_by_name: BTreeMap<String, RegisteredCallback>,\n",
        "    handles_to_name: BTreeMap<u64, String>,\n",
        "    next_handle: u64,\n",
        "    package_state: PackageState,\n",
        "}\n\n",
        "fn binding_tag(value: &ArcanaCabiBindingValueV1) -> Result<ArcanaCabiBindingValueTag, String> {\n",
        "    value.tag()\n",
        "}\n\n",
        "fn binding_int(value: i64) -> ArcanaCabiBindingValueV1 {\n",
        "    ArcanaCabiBindingValueV1 {\n",
        "        tag: ArcanaCabiBindingValueTag::Int as u32,\n",
        "        payload: ArcanaCabiBindingPayloadV1 { int_value: value },\n",
        "        ..ArcanaCabiBindingValueV1::default()\n",
        "    }\n",
        "}\n\n",
        "fn binding_bool(value: bool) -> ArcanaCabiBindingValueV1 {\n",
        "    ArcanaCabiBindingValueV1 {\n",
        "        tag: ArcanaCabiBindingValueTag::Bool as u32,\n",
        "        payload: ArcanaCabiBindingPayloadV1 { bool_value: u8::from(value) },\n",
        "        ..ArcanaCabiBindingValueV1::default()\n",
        "    }\n",
        "}\n\n",
        "fn binding_i8(value: i8) -> ArcanaCabiBindingValueV1 {\n",
        "    ArcanaCabiBindingValueV1 {\n",
        "        tag: ArcanaCabiBindingValueTag::I8 as u32,\n",
        "        payload: ArcanaCabiBindingPayloadV1 { i8_value: value },\n",
        "        ..ArcanaCabiBindingValueV1::default()\n",
        "    }\n",
        "}\n\n",
        "fn binding_u8(value: u8) -> ArcanaCabiBindingValueV1 {\n",
        "    ArcanaCabiBindingValueV1 {\n",
        "        tag: ArcanaCabiBindingValueTag::U8 as u32,\n",
        "        payload: ArcanaCabiBindingPayloadV1 { u8_value: value },\n",
        "        ..ArcanaCabiBindingValueV1::default()\n",
        "    }\n",
        "}\n\n",
        "fn binding_i16(value: i16) -> ArcanaCabiBindingValueV1 {\n",
        "    ArcanaCabiBindingValueV1 {\n",
        "        tag: ArcanaCabiBindingValueTag::I16 as u32,\n",
        "        payload: ArcanaCabiBindingPayloadV1 { i16_value: value },\n",
        "        ..ArcanaCabiBindingValueV1::default()\n",
        "    }\n",
        "}\n\n",
        "fn binding_u16(value: u16) -> ArcanaCabiBindingValueV1 {\n",
        "    ArcanaCabiBindingValueV1 {\n",
        "        tag: ArcanaCabiBindingValueTag::U16 as u32,\n",
        "        payload: ArcanaCabiBindingPayloadV1 { u16_value: value },\n",
        "        ..ArcanaCabiBindingValueV1::default()\n",
        "    }\n",
        "}\n\n",
        "fn binding_i32(value: i32) -> ArcanaCabiBindingValueV1 {\n",
        "    ArcanaCabiBindingValueV1 {\n",
        "        tag: ArcanaCabiBindingValueTag::I32 as u32,\n",
        "        payload: ArcanaCabiBindingPayloadV1 { i32_value: value },\n",
        "        ..ArcanaCabiBindingValueV1::default()\n",
        "    }\n",
        "}\n\n",
        "fn binding_u32(value: u32) -> ArcanaCabiBindingValueV1 {\n",
        "    ArcanaCabiBindingValueV1 {\n",
        "        tag: ArcanaCabiBindingValueTag::U32 as u32,\n",
        "        payload: ArcanaCabiBindingPayloadV1 { u32_value: value },\n",
        "        ..ArcanaCabiBindingValueV1::default()\n",
        "    }\n",
        "}\n\n",
        "fn binding_i64(value: i64) -> ArcanaCabiBindingValueV1 {\n",
        "    ArcanaCabiBindingValueV1 {\n",
        "        tag: ArcanaCabiBindingValueTag::I64 as u32,\n",
        "        payload: ArcanaCabiBindingPayloadV1 { i64_value: value },\n",
        "        ..ArcanaCabiBindingValueV1::default()\n",
        "    }\n",
        "}\n\n",
        "fn binding_u64(value: u64) -> ArcanaCabiBindingValueV1 {\n",
        "    ArcanaCabiBindingValueV1 {\n",
        "        tag: ArcanaCabiBindingValueTag::U64 as u32,\n",
        "        payload: ArcanaCabiBindingPayloadV1 { u64_value: value },\n",
        "        ..ArcanaCabiBindingValueV1::default()\n",
        "    }\n",
        "}\n\n",
        "fn binding_isize(value: isize) -> ArcanaCabiBindingValueV1 {\n",
        "    ArcanaCabiBindingValueV1 {\n",
        "        tag: ArcanaCabiBindingValueTag::ISize as u32,\n",
        "        payload: ArcanaCabiBindingPayloadV1 { isize_value: value },\n",
        "        ..ArcanaCabiBindingValueV1::default()\n",
        "    }\n",
        "}\n\n",
        "fn binding_usize(value: usize) -> ArcanaCabiBindingValueV1 {\n",
        "    ArcanaCabiBindingValueV1 {\n",
        "        tag: ArcanaCabiBindingValueTag::USize as u32,\n",
        "        payload: ArcanaCabiBindingPayloadV1 { usize_value: value },\n",
        "        ..ArcanaCabiBindingValueV1::default()\n",
        "    }\n",
        "}\n\n",
        "fn binding_f32(value: f32) -> ArcanaCabiBindingValueV1 {\n",
        "    ArcanaCabiBindingValueV1 {\n",
        "        tag: ArcanaCabiBindingValueTag::F32 as u32,\n",
        "        payload: ArcanaCabiBindingPayloadV1 { f32_value: value },\n",
        "        ..ArcanaCabiBindingValueV1::default()\n",
        "    }\n",
        "}\n\n",
        "fn binding_f64(value: f64) -> ArcanaCabiBindingValueV1 {\n",
        "    ArcanaCabiBindingValueV1 {\n",
        "        tag: ArcanaCabiBindingValueTag::F64 as u32,\n",
        "        payload: ArcanaCabiBindingPayloadV1 { f64_value: value },\n",
        "        ..ArcanaCabiBindingValueV1::default()\n",
        "    }\n",
        "}\n\n",
        "fn binding_layout<T: Copy>(value: T) -> ArcanaCabiBindingValueV1 {\n",
        "    let len = std::mem::size_of::<T>();\n",
        "    let bytes = if len == 0 {\n",
        "        Vec::new()\n",
        "    } else {\n",
        "        unsafe { std::slice::from_raw_parts((&value as *const T).cast::<u8>(), len) }.to_vec()\n",
        "    };\n",
        "    ArcanaCabiBindingValueV1 {\n",
        "        tag: ArcanaCabiBindingValueTag::Layout as u32,\n",
        "        payload: ArcanaCabiBindingPayloadV1 { owned_bytes_value: into_owned_bytes(bytes) },\n",
        "        ..ArcanaCabiBindingValueV1::default()\n",
        "    }\n",
        "}\n\n",
        "fn binding_owned_str(text: String) -> ArcanaCabiBindingValueV1 {\n",
        "    ArcanaCabiBindingValueV1 {\n",
        "        tag: ArcanaCabiBindingValueTag::Str as u32,\n",
        "        payload: ArcanaCabiBindingPayloadV1 { owned_str_value: into_owned_str(text) },\n",
        "        ..ArcanaCabiBindingValueV1::default()\n",
        "    }\n",
        "}\n\n",
        "fn binding_owned_bytes(bytes: Vec<u8>) -> ArcanaCabiBindingValueV1 {\n",
        "    ArcanaCabiBindingValueV1 {\n",
        "        tag: ArcanaCabiBindingValueTag::Bytes as u32,\n",
        "        payload: ArcanaCabiBindingPayloadV1 { owned_bytes_value: into_owned_bytes(bytes) },\n",
        "        ..ArcanaCabiBindingValueV1::default()\n",
        "    }\n",
        "}\n\n",
        "fn binding_unit() -> ArcanaCabiBindingValueV1 {\n",
        "    ArcanaCabiBindingValueV1::default()\n",
        "}\n\n",
        "fn require_arg_count(actual: usize, expected: usize, import_name: &str) -> Result<(), String> {\n",
        "    if actual == expected {\n",
        "        Ok(())\n",
        "    } else {\n",
        "        Err(format!(\"binding import `{import_name}` expected {expected} args, got {actual}\"))\n",
        "    }\n",
        "}\n\n",
        "unsafe fn instance_ptr(instance: *mut c_void) -> Result<*mut BindingInstance, String> {\n",
        "    if instance.is_null() {\n",
        "        Err(\"binding instance must not be null\".to_string())\n",
        "    } else {\n",
        "        Ok(instance.cast())\n",
        "    }\n",
        "}\n\n",
        "fn read_int_arg(value: &ArcanaCabiBindingValueV1, name: &str) -> Result<i64, String> {\n",
        "    if binding_tag(value)? != ArcanaCabiBindingValueTag::Int {\n",
        "        return Err(format!(\"binding arg `{name}` must be Int\"));\n",
        "    }\n",
        "    Ok(unsafe { value.payload.int_value })\n",
        "}\n\n",
        "fn read_bool_arg(value: &ArcanaCabiBindingValueV1, name: &str) -> Result<bool, String> {\n",
        "    if binding_tag(value)? != ArcanaCabiBindingValueTag::Bool {\n",
        "        return Err(format!(\"binding arg `{name}` must be Bool\"));\n",
        "    }\n",
        "    Ok(unsafe { value.payload.bool_value != 0 })\n",
        "}\n\n",
        "fn read_i8_arg(value: &ArcanaCabiBindingValueV1, name: &str) -> Result<i8, String> {\n",
        "    if binding_tag(value)? != ArcanaCabiBindingValueTag::I8 {\n",
        "        return Err(format!(\"binding arg `{name}` must be I8\"));\n",
        "    }\n",
        "    Ok(unsafe { value.payload.i8_value })\n",
        "}\n\n",
        "fn read_u8_arg(value: &ArcanaCabiBindingValueV1, name: &str) -> Result<u8, String> {\n",
        "    if binding_tag(value)? != ArcanaCabiBindingValueTag::U8 {\n",
        "        return Err(format!(\"binding arg `{name}` must be U8\"));\n",
        "    }\n",
        "    Ok(unsafe { value.payload.u8_value })\n",
        "}\n\n",
        "fn read_i16_arg(value: &ArcanaCabiBindingValueV1, name: &str) -> Result<i16, String> {\n",
        "    if binding_tag(value)? != ArcanaCabiBindingValueTag::I16 {\n",
        "        return Err(format!(\"binding arg `{name}` must be I16\"));\n",
        "    }\n",
        "    Ok(unsafe { value.payload.i16_value })\n",
        "}\n\n",
        "fn read_u16_arg(value: &ArcanaCabiBindingValueV1, name: &str) -> Result<u16, String> {\n",
        "    if binding_tag(value)? != ArcanaCabiBindingValueTag::U16 {\n",
        "        return Err(format!(\"binding arg `{name}` must be U16\"));\n",
        "    }\n",
        "    Ok(unsafe { value.payload.u16_value })\n",
        "}\n\n",
        "fn read_i32_arg(value: &ArcanaCabiBindingValueV1, name: &str) -> Result<i32, String> {\n",
        "    if binding_tag(value)? != ArcanaCabiBindingValueTag::I32 {\n",
        "        return Err(format!(\"binding arg `{name}` must be I32\"));\n",
        "    }\n",
        "    Ok(unsafe { value.payload.i32_value })\n",
        "}\n\n",
        "fn read_u32_arg(value: &ArcanaCabiBindingValueV1, name: &str) -> Result<u32, String> {\n",
        "    if binding_tag(value)? != ArcanaCabiBindingValueTag::U32 {\n",
        "        return Err(format!(\"binding arg `{name}` must be U32\"));\n",
        "    }\n",
        "    Ok(unsafe { value.payload.u32_value })\n",
        "}\n\n",
        "fn read_i64_arg(value: &ArcanaCabiBindingValueV1, name: &str) -> Result<i64, String> {\n",
        "    if binding_tag(value)? != ArcanaCabiBindingValueTag::I64 {\n",
        "        return Err(format!(\"binding arg `{name}` must be I64\"));\n",
        "    }\n",
        "    Ok(unsafe { value.payload.i64_value })\n",
        "}\n\n",
        "fn read_u64_arg(value: &ArcanaCabiBindingValueV1, name: &str) -> Result<u64, String> {\n",
        "    if binding_tag(value)? != ArcanaCabiBindingValueTag::U64 {\n",
        "        return Err(format!(\"binding arg `{name}` must be U64\"));\n",
        "    }\n",
        "    Ok(unsafe { value.payload.u64_value })\n",
        "}\n\n",
        "fn read_isize_arg(value: &ArcanaCabiBindingValueV1, name: &str) -> Result<isize, String> {\n",
        "    if binding_tag(value)? != ArcanaCabiBindingValueTag::ISize {\n",
        "        return Err(format!(\"binding arg `{name}` must be ISize\"));\n",
        "    }\n",
        "    Ok(unsafe { value.payload.isize_value })\n",
        "}\n\n",
        "fn read_usize_arg(value: &ArcanaCabiBindingValueV1, name: &str) -> Result<usize, String> {\n",
        "    if binding_tag(value)? != ArcanaCabiBindingValueTag::USize {\n",
        "        return Err(format!(\"binding arg `{name}` must be USize\"));\n",
        "    }\n",
        "    Ok(unsafe { value.payload.usize_value })\n",
        "}\n\n",
        "fn read_f32_arg(value: &ArcanaCabiBindingValueV1, name: &str) -> Result<f32, String> {\n",
        "    if binding_tag(value)? != ArcanaCabiBindingValueTag::F32 {\n",
        "        return Err(format!(\"binding arg `{name}` must be F32\"));\n",
        "    }\n",
        "    Ok(unsafe { value.payload.f32_value })\n",
        "}\n\n",
        "fn read_f64_arg(value: &ArcanaCabiBindingValueV1, name: &str) -> Result<f64, String> {\n",
        "    if binding_tag(value)? != ArcanaCabiBindingValueTag::F64 {\n",
        "        return Err(format!(\"binding arg `{name}` must be F64\"));\n",
        "    }\n",
        "    Ok(unsafe { value.payload.f64_value })\n",
        "}\n\n",
        "fn read_unit_arg(value: &ArcanaCabiBindingValueV1, name: &str) -> Result<(), String> {\n",
        "    if binding_tag(value)? != ArcanaCabiBindingValueTag::Unit {\n",
        "        return Err(format!(\"binding arg `{name}` must be Unit\"));\n",
        "    }\n",
        "    Ok(())\n",
        "}\n\n",
        "fn read_layout_arg<T: Copy>(value: &ArcanaCabiBindingValueV1, name: &str) -> Result<T, String> {\n",
        "    if binding_tag(value)? != ArcanaCabiBindingValueTag::Layout {\n",
        "        return Err(format!(\"binding arg `{name}` must be Layout\"));\n",
        "    }\n",
        "    let view = unsafe { value.payload.bytes_value };\n",
        "    let expected_len = std::mem::size_of::<T>();\n",
        "    if view.len != expected_len {\n",
        "        return Err(format!(\"binding arg `{name}` layout size mismatch: expected {expected_len}, got {}\", view.len));\n",
        "    }\n",
        "    if view.ptr.is_null() {\n",
        "        if expected_len == 0 {\n",
        "            return Ok(unsafe { std::mem::zeroed() });\n",
        "        }\n",
        "        return Err(format!(\"binding arg `{name}` returned null Layout data with len {}\", view.len));\n",
        "    }\n",
        "    if (view.ptr as usize) % std::mem::align_of::<T>() == 0 {\n",
        "        Ok(unsafe { *(view.ptr.cast::<T>()) })\n",
        "    } else {\n",
        "        Ok(unsafe { std::ptr::read_unaligned(view.ptr.cast::<T>()) })\n",
        "    }\n",
        "}\n\n",
        "fn read_utf8_arg(value: &ArcanaCabiBindingValueV1, name: &str) -> Result<String, String> {\n",
        "    if binding_tag(value)? != ArcanaCabiBindingValueTag::Str {\n",
        "        return Err(format!(\"binding arg `{name}` must be Str\"));\n",
        "    }\n",
        "    let view = unsafe { value.payload.str_value };\n",
        "    let bytes = if view.ptr.is_null() {\n",
        "        if view.len == 0 { &[][..] } else {\n",
        "            return Err(format!(\"binding arg `{name}` returned null Str data with len {}\", view.len));\n",
        "        }\n",
        "    } else {\n",
        "        unsafe { std::slice::from_raw_parts(view.ptr, view.len) }\n",
        "    };\n",
        "    String::from_utf8(bytes.to_vec()).map_err(|err| format!(\"binding arg `{name}` is not utf-8: {err}\"))\n",
        "}\n\n",
        "fn read_bytes_arg(value: &ArcanaCabiBindingValueV1, name: &str) -> Result<Vec<u8>, String> {\n",
        "    if binding_tag(value)? != ArcanaCabiBindingValueTag::Bytes {\n",
        "        return Err(format!(\"binding arg `{name}` must be Bytes\"));\n",
        "    }\n",
        "    let view = unsafe { value.payload.bytes_value };\n",
        "    let bytes = if view.ptr.is_null() {\n",
        "        if view.len == 0 { &[][..] } else {\n",
        "            return Err(format!(\"binding arg `{name}` returned null Bytes data with len {}\", view.len));\n",
        "        }\n",
        "    } else {\n",
        "        unsafe { std::slice::from_raw_parts(view.ptr, view.len) }\n",
        "    };\n",
        "    Ok(bytes.to_vec())\n",
        "}\n\n",
        "unsafe fn invoke_callback_value_result(\n",
        "    instance: &mut BindingInstance,\n",
        "    callback_name: &str,\n",
        "    args: &[ArcanaCabiBindingValueV1],\n",
        ") -> Result<ArcanaCabiBindingValueV1, String> {\n",
        "    let callback = instance\n",
        "        .callbacks_by_name\n",
        "        .get(callback_name)\n",
        "        .copied()\n",
        "        .ok_or_else(|| format!(\"no registered `{callback_name}` callback is active\"))?;\n",
        "    let mut write_backs = vec![ArcanaCabiBindingValueV1::default(); args.len()];\n",
        "    let mut out = ArcanaCabiBindingValueV1::default();\n",
        "    let ok = unsafe {\n",
        "        (callback.callback)(\n",
        "            callback.user_data,\n",
        "            args.as_ptr(),\n",
        "            args.len(),\n",
        "            write_backs.as_mut_ptr(),\n",
        "            &mut out,\n",
        "        )\n",
        "    };\n",
        "    if ok == 0 {\n",
        "        for value in write_backs {\n",
        "            let _ = release_binding_output_value(value, callback.owned_bytes_free, callback.owned_str_free);\n",
        "        }\n",
        "        let _ = release_binding_output_value(out, callback.owned_bytes_free, callback.owned_str_free);\n",
        "        return Err(format!(\"registered `{callback_name}` callback returned failure\"));\n",
        "    }\n",
        "    for value in write_backs {\n",
        "        let _ = release_binding_output_value(value, callback.owned_bytes_free, callback.owned_str_free);\n",
        "    }\n",
        "    Ok(out)\n",
        "}\n\n",
        "unsafe fn invoke_callback_int_result(\n",
        "    instance: &mut BindingInstance,\n",
        "    callback_name: &str,\n",
        "    args: &[ArcanaCabiBindingValueV1],\n",
        ") -> Result<i64, String> {\n",
        "    let out = unsafe { invoke_callback_value_result(instance, callback_name, args) }?;\n",
        "    if binding_tag(&out)? != ArcanaCabiBindingValueTag::Int {\n",
        "        let callback = instance\n",
        "            .callbacks_by_name\n",
        "            .get(callback_name)\n",
        "            .copied()\n",
        "            .ok_or_else(|| format!(\"no registered `{callback_name}` callback is active\"))?;\n",
        "        let _ = release_binding_output_value(out, callback.owned_bytes_free, callback.owned_str_free);\n",
        "        return Err(format!(\"registered `{callback_name}` callback returned a non-Int result\"));\n",
        "    }\n",
        "    Ok(unsafe { out.payload.int_value })\n",
        "}\n\n",
        "unsafe extern \"system\" fn create_binding_instance() -> *mut c_void {\n",
        "    match package_state_init() {\n",
        "        Ok(package_state) => Box::into_raw(Box::new(BindingInstance {\n",
        "            callbacks_by_name: BTreeMap::new(),\n",
        "            handles_to_name: BTreeMap::new(),\n",
        "            next_handle: 1,\n",
        "            package_state,\n",
        "        })) as *mut c_void,\n",
        "        Err(err) => {\n",
        "            set_last_error(err);\n",
        "            ptr::null_mut()\n",
        "        }\n",
        "    }\n",
        "}\n\n",
        "unsafe extern \"system\" fn destroy_binding_instance(instance: *mut c_void) {\n",
        "    if instance.is_null() {\n",
        "        return;\n",
        "    }\n",
        "    let mut instance = unsafe { Box::from_raw(instance as *mut BindingInstance) };\n",
        "    package_state_drop(&mut instance.package_state);\n",
        "}\n\n",
        "unsafe extern \"system\" fn register_callback(\n",
        "    instance: *mut c_void,\n",
        "    callback_name: *const c_char,\n",
        "    callback: ArcanaCabiBindingCallbackFn,\n",
        "    callback_owned_bytes_free: ArcanaCabiOwnedBytesFreeFn,\n",
        "    callback_owned_str_free: ArcanaCabiOwnedStrFreeFn,\n",
        "    user_data: *mut c_void,\n",
        "    out_handle: *mut u64,\n",
        ") -> i32 {\n",
        "    let result = (|| -> Result<(), String> {\n",
        "        let instance = unsafe { &mut *instance_ptr(instance)? };\n",
        "        if callback_name.is_null() {\n",
        "            return Err(\"binding callback name must not be null\".to_string());\n",
        "        }\n",
        "        let name = unsafe { CStr::from_ptr(callback_name) }\n",
        "            .to_str()\n",
        "            .map_err(|err| format!(\"binding callback name is not utf-8: {err}\"))?\n",
        "            .to_string();\n",
        "        if !binding_callback_name_is_declared(&name) {\n",
        "            return Err(format!(\"binding callback `{name}` is not declared by this product\"));\n",
        "        }\n",
        "        if instance.callbacks_by_name.contains_key(&name) {\n",
        "            return Err(format!(\"binding callback `{name}` is already registered\"));\n",
        "        }\n",
        "        let handle = instance.next_handle;\n",
        "        instance.next_handle += 1;\n",
        "        instance.callbacks_by_name.insert(\n",
        "            name.clone(),\n",
        "            RegisteredCallback {\n",
        "                callback,\n",
        "                owned_bytes_free: callback_owned_bytes_free,\n",
        "                owned_str_free: callback_owned_str_free,\n",
        "                user_data,\n",
        "            },\n",
        "        );\n",
        "        instance.handles_to_name.insert(handle, name);\n",
        "        if !out_handle.is_null() {\n",
        "            unsafe { *out_handle = handle; }\n",
        "        }\n",
        "        Ok(())\n",
        "    })();\n",
        "    match result {\n",
        "        Ok(()) => 1,\n",
        "        Err(err) => {\n",
        "            set_last_error(err);\n",
        "            0\n",
        "        }\n",
        "    }\n",
        "}\n\n",
        "unsafe extern \"system\" fn unregister_callback(instance: *mut c_void, handle: u64) -> i32 {\n",
        "    let result = (|| -> Result<(), String> {\n",
        "        let instance = unsafe { &mut *instance_ptr(instance)? };\n",
        "        let name = instance\n",
        "            .handles_to_name\n",
        "            .remove(&handle)\n",
        "            .ok_or_else(|| format!(\"binding callback handle `{handle}` is not active\"))?;\n",
        "        instance.callbacks_by_name.remove(&name);\n",
        "        Ok(())\n",
        "    })();\n",
        "    match result {\n",
        "        Ok(()) => 1,\n",
        "        Err(err) => {\n",
        "            set_last_error(err);\n",
        "            0\n",
        "        }\n",
        "    }\n",
        "}\n\n",
        "unsafe fn run_binding_import(\n",
        "    import_name: &str,\n",
        "    instance: *mut c_void,\n",
        "    args: *const ArcanaCabiBindingValueV1,\n",
        "    arg_count: usize,\n",
        "    out_write_backs: *mut ArcanaCabiBindingValueV1,\n",
        "    out_result: *mut ArcanaCabiBindingValueV1,\n",
        "    handler: fn(\n",
        "        &mut BindingInstance,\n",
        "        &[ArcanaCabiBindingValueV1],\n",
        "        &mut [ArcanaCabiBindingValueV1],\n",
        "    ) -> Result<ArcanaCabiBindingValueV1, String>,\n",
        ") -> i32 {\n",
        "    let result = (|| -> Result<(), String> {\n",
        "        if out_result.is_null() {\n",
        "            return Err(format!(\"binding import `{import_name}` requires non-null out_result\"));\n",
        "        }\n",
        "        if args.is_null() && arg_count != 0 {\n",
        "            return Err(format!(\"binding import `{import_name}` received null args with non-zero count\"));\n",
        "        }\n",
        "        if out_write_backs.is_null() && arg_count != 0 {\n",
        "            return Err(format!(\"binding import `{import_name}` requires non-null out_write_backs when args are present\"));\n",
        "        }\n",
        "        let instance = unsafe { &mut *instance_ptr(instance)? };\n",
        "        let args = if arg_count == 0 {\n",
        "            &[][..]\n",
        "        } else {\n",
        "            unsafe { std::slice::from_raw_parts(args, arg_count) }\n",
        "        };\n",
        "        let write_backs = if arg_count == 0 {\n",
        "            &mut [][..]\n",
        "        } else {\n",
        "            unsafe { std::slice::from_raw_parts_mut(out_write_backs, arg_count) }\n",
        "        };\n",
        "        for slot in write_backs.iter_mut() {\n",
        "            *slot = ArcanaCabiBindingValueV1::default();\n",
        "        }\n",
        "        let value = handler(instance, args, write_backs)?;\n",
        "        unsafe { *out_result = value; }\n",
        "        Ok(())\n",
        "    })();\n",
        "    match result {\n",
        "        Ok(()) => 1,\n",
        "        Err(err) => {\n",
        "            set_last_error(err);\n",
        "            0\n",
        "        }\n",
        "    }\n",
        "}\n\n",
    )
    .to_string()
}

fn render_generated_binding_descriptor() -> &'static str {
    concat!(
        "static BINDING_OPS: ArcanaCabiBindingOpsV1 = ArcanaCabiBindingOpsV1 {\n",
        "    base: ArcanaCabiInstanceOpsV1 {\n",
        "        ops_size: std::mem::size_of::<ArcanaCabiInstanceOpsV1>(),\n",
        "        create_instance: create_binding_instance as ArcanaCabiCreateInstanceFn,\n",
        "        destroy_instance: destroy_binding_instance as ArcanaCabiDestroyInstanceFn,\n",
        "        reserved0: ptr::null(),\n",
        "        reserved1: ptr::null(),\n",
        "    },\n",
        "    imports: BINDING_IMPORTS.as_ptr(),\n",
        "    import_count: BINDING_IMPORTS.len(),\n",
        "    callbacks: BINDING_CALLBACKS.as_ptr(),\n",
        "    callback_count: BINDING_CALLBACKS.len(),\n",
        "    layouts: BINDING_LAYOUTS.as_ptr(),\n",
        "    layout_count: BINDING_LAYOUTS.len(),\n",
        "    register_callback: register_callback as ArcanaCabiBindingRegisterCallbackFn,\n",
        "    unregister_callback: unregister_callback as ArcanaCabiBindingUnregisterCallbackFn,\n",
        "    last_error_alloc: binding_last_error_alloc as ArcanaCabiLastErrorAllocFn,\n",
        "    owned_bytes_free: binding_owned_bytes_free as ArcanaCabiOwnedBytesFreeFn,\n",
        "    owned_str_free: binding_owned_str_free as ArcanaCabiOwnedStrFreeFn,\n",
        "    reserved0: ptr::null(),\n",
        "    reserved1: ptr::null(),\n",
        "};\n\n",
        "static PRODUCT_API: ArcanaCabiProductApiV1 = ArcanaCabiProductApiV1 {\n",
        "    descriptor_size: std::mem::size_of::<ArcanaCabiProductApiV1>(),\n",
        "    package_name: PACKAGE_NAME.as_ptr() as *const c_char,\n",
        "    product_name: PRODUCT_NAME.as_ptr() as *const c_char,\n",
        "    role: ROLE_NAME.as_ptr() as *const c_char,\n",
        "    contract_id: CONTRACT_ID.as_ptr() as *const c_char,\n",
        "    contract_version: ARCANA_CABI_CONTRACT_VERSION_V1,\n",
        "    role_ops: &BINDING_OPS as *const ArcanaCabiBindingOpsV1 as *const c_void,\n",
        "    reserved0: ptr::null(),\n",
        "    reserved1: ptr::null(),\n",
        "};\n\n",
        "const _: &str = ARCANA_CABI_GET_PRODUCT_API_V1_SYMBOL;\n",
        "const _: u32 = ARCANA_CABI_CONTRACT_VERSION_V1;\n\n",
        "#[unsafe(no_mangle)]\n",
        "pub extern \"system\" fn arcana_cabi_get_product_api_v1() -> *const ArcanaCabiProductApiV1 {\n",
        "    &PRODUCT_API\n",
        "}\n"
    )
}

fn render_shackle_support_items(spec: &AotInstanceProductSpec) -> Result<String, String> {
    let mut out = String::new();
    if let Some(alias) = render_package_state_alias(spec)? {
        out.push_str(&alias);
    }
    if let Some(init) = render_package_state_init(spec)? {
        out.push_str(&init);
    }
    if let Some(drop_fn) = render_package_state_drop(spec)? {
        out.push_str(&drop_fn);
    }
    let tree = build_shackle_module_tree(spec);
    out.push_str(&render_shackle_module_items(spec, &tree, 0)?);
    Ok(out)
}

fn render_shackle_import_fn_decl(
    spec: &AotInstanceProductSpec,
    decl: &AotShackleDeclArtifact,
) -> Result<String, String> {
    if !decl.body_entries.is_empty() {
        let mut out = String::new();
        for line in &decl.body_entries {
            out.push_str(line);
            out.push('\n');
        }
        out.push('\n');
        return Ok(out);
    }
    let import_target = decl.import_target.as_ref().ok_or_else(|| {
        format!(
            "shackle import fn `{}` is missing a typed import target in generated binding product",
            decl.name
        )
    })?;
    Ok(format!(
        "#[link(name = {:?})]\nunsafe extern {:?} {{\n    #[link_name = {:?}]\n    pub fn {}({}){};\n}}\n\n",
        import_target.library,
        import_target.abi,
        import_target.symbol,
        decl.name,
        render_shackle_rust_params(spec, &decl.params),
        render_shackle_rust_return_type(spec, decl.return_type.as_ref())
    ))
}

fn render_shackle_const_decl(
    spec: &AotInstanceProductSpec,
    decl: &AotShackleDeclArtifact,
) -> Result<String, String> {
    if !decl.body_entries.is_empty() {
        let mut out = String::new();
        for line in &decl.body_entries {
            out.push_str(line);
            out.push('\n');
        }
        out.push('\n');
        return Ok(out);
    }
    let binding = decl.binding.as_deref().ok_or_else(|| {
        format!(
            "shackle const `{}` is missing a binding expression in generated binding product",
            decl.name
        )
    })?;
    Ok(format!(
        "pub(crate) const {}: {} = {};\n\n",
        decl.name,
        decl.return_type
            .as_ref()
            .map(|ty| render_shackle_rust_type(spec, ty))
            .unwrap_or_else(|| "()".to_string()),
        rewrite_shackle_expr_binding(spec, binding)
    ))
}

fn render_shackle_raw_decl(
    spec: &AotInstanceProductSpec,
    decl: &AotShackleDeclArtifact,
) -> Result<String, String> {
    if let Some(layout) = decl.raw_layout.as_ref() {
        return render_shackle_typed_raw_decl(spec, decl, layout);
    }
    if !decl.body_entries.is_empty() {
        let mut out = String::new();
        match decl.kind.as_str() {
            "struct" => {
                out.push_str(&format!(
                    "#[derive(Clone, Copy)]\n#[repr(C)]\npub(crate) struct {} {{\n",
                    decl.name
                ));
                for line in &decl.body_entries {
                    out.push_str("    ");
                    out.push_str(&render_shackle_struct_field(line));
                    out.push('\n');
                }
                out.push_str("}\n\n");
            }
            "union" => {
                out.push_str(&format!(
                    "#[derive(Clone, Copy)]\n#[repr(C)]\npub(crate) union {} {{\n",
                    decl.name
                ));
                for line in &decl.body_entries {
                    out.push_str("    ");
                    out.push_str(&render_shackle_struct_field(line));
                    out.push('\n');
                }
                out.push_str("}\n\n");
            }
            _ => {
                for line in &decl.body_entries {
                    out.push_str(line);
                    out.push('\n');
                }
                out.push('\n');
            }
        }
        return Ok(out);
    }
    let binding = decl.binding.as_deref().ok_or_else(|| {
        format!(
            "shackle {} `{}` must either provide a body or a binding target",
            decl.kind, decl.name
        )
    })?;
    let rendered_binding = if decl.kind == "type" {
        arcana_ir::parse_routine_type_text(binding)
            .map(|ty| render_shackle_rust_type(spec, &ty))
            .unwrap_or_else(|_| rewrite_shackle_type_binding(spec, binding))
    } else {
        binding.to_string()
    };
    Ok(format!(
        "pub(crate) type {} = {};\n\n",
        decl.name, rendered_binding
    ))
}

fn rewrite_shackle_type_binding(spec: &AotInstanceProductSpec, binding: &str) -> String {
    let package_prefix = format!("{}.", spec.package_name);
    binding
        .replace(&package_prefix, "crate::")
        .replace("c_void", "std::ffi::c_void")
        .replace('.', "::")
}

fn rewrite_shackle_expr_binding(spec: &AotInstanceProductSpec, binding: &str) -> String {
    let trimmed = binding.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if !trimmed.contains(&format!("{}.", spec.package_name)) {
        return trimmed.to_string();
    }
    let segments = trimmed.split('.').collect::<Vec<_>>();
    for split in (1..=segments.len()).rev() {
        let candidate = segments[..split].join(".");
        if spec
            .binding_shackle_decls
            .iter()
            .any(|decl| format!("{}.{}", decl.module_id, decl.name) == candidate)
        {
            let mut rendered = rewrite_shackle_type_binding(spec, &candidate);
            if split < segments.len() {
                rendered.push('.');
                rendered.push_str(&segments[split..].join("."));
            }
            return rendered;
        }
    }
    rewrite_shackle_type_binding(spec, trimmed)
}

fn render_shackle_typed_raw_decl(
    spec: &AotInstanceProductSpec,
    decl: &AotShackleDeclArtifact,
    layout: &arcana_cabi::ArcanaCabiBindingLayout,
) -> Result<String, String> {
    use arcana_cabi::ArcanaCabiBindingLayoutKind;

    Ok(match &layout.kind {
        ArcanaCabiBindingLayoutKind::Alias { target } => format!(
            "pub(crate) type {} = {};\n\n",
            decl.name,
            render_shackle_binding_raw_type(spec, target)
        ),
        ArcanaCabiBindingLayoutKind::Array { element_type, len } => format!(
            "pub(crate) type {} = [{}; {}];\n\n",
            decl.name,
            render_shackle_binding_raw_type(spec, element_type),
            len
        ),
        ArcanaCabiBindingLayoutKind::Enum { repr, variants } => {
            let value_set_ty = format!("{}__ValueSet", decl.name);
            let mut out = String::new();
            out.push_str(&format!(
                "pub(crate) type {} = {};\n\n",
                decl.name,
                render_shackle_binding_scalar_type(*repr)
            ));
            out.push_str("#[allow(non_snake_case)]\n");
            out.push_str(&format!("pub(crate) struct {value_set_ty} {{\n"));
            for variant in variants {
                out.push_str(&format!(
                    "    pub(crate) {}: {},\n",
                    variant.name, decl.name
                ));
            }
            out.push_str("}\n\n");
            out.push_str("#[allow(non_upper_case_globals)]\n");
            out.push_str(&format!(
                "pub(crate) const {}: {} = {} {{\n",
                decl.name, value_set_ty, value_set_ty
            ));
            for variant in variants {
                out.push_str(&format!(
                    "    {}: {} as {},\n",
                    variant.name, variant.value, decl.name
                ));
            }
            out.push_str("};\n\n");
            out
        }
        ArcanaCabiBindingLayoutKind::Flags { repr } => format!(
            "pub(crate) type {} = {};\n\n",
            decl.name,
            render_shackle_binding_scalar_type(*repr)
        ),
        ArcanaCabiBindingLayoutKind::Struct { fields } => {
            let mut out = String::new();
            out.push_str("#[allow(non_snake_case)]\n");
            out.push_str(&format!(
                "#[derive(Clone, Copy)]\n#[repr(C)]\npub(crate) struct {} {{\n",
                decl.name
            ));
            for field in fields {
                out.push_str(&format!(
                    "    pub(crate) {}: {},\n",
                    field.name,
                    render_shackle_binding_raw_type(spec, &field.ty)
                ));
            }
            out.push_str("}\n\n");
            out
        }
        ArcanaCabiBindingLayoutKind::Union { fields } => {
            let mut out = String::new();
            out.push_str("#[allow(non_snake_case)]\n");
            out.push_str(&format!(
                "#[derive(Clone, Copy)]\n#[repr(C)]\npub(crate) union {} {{\n",
                decl.name
            ));
            for field in fields {
                out.push_str(&format!(
                    "    pub(crate) {}: {},\n",
                    field.name,
                    render_shackle_binding_raw_type(spec, &field.ty)
                ));
            }
            out.push_str("}\n\n");
            out
        }
        ArcanaCabiBindingLayoutKind::Callback {
            abi,
            params,
            return_type,
        } => format!(
            "pub(crate) type {} = {};\n\n",
            decl.name,
            render_shackle_binding_function_pointer_type(spec, abi, true, params, return_type)
        ),
        ArcanaCabiBindingLayoutKind::Interface { .. } => {
            let rendered_binding = decl
                .binding
                .as_deref()
                .map(|binding| rewrite_shackle_type_binding(spec, binding))
                .unwrap_or_else(|| "*mut std::ffi::c_void".to_string());
            format!("pub(crate) type {} = {};\n\n", decl.name, rendered_binding)
        }
    })
}

fn render_shackle_binding_raw_type(
    spec: &AotInstanceProductSpec,
    ty: &arcana_cabi::ArcanaCabiBindingRawType,
) -> String {
    use arcana_cabi::ArcanaCabiBindingRawType;

    match ty {
        ArcanaCabiBindingRawType::Void => "std::ffi::c_void".to_string(),
        ArcanaCabiBindingRawType::Scalar(scalar) => render_shackle_binding_scalar_type(*scalar),
        ArcanaCabiBindingRawType::Named(name) => rewrite_shackle_type_binding(spec, name),
        ArcanaCabiBindingRawType::Pointer { mutable, inner } => format!(
            "*{} {}",
            if *mutable { "mut" } else { "const" },
            render_shackle_binding_raw_type(spec, inner)
        ),
        ArcanaCabiBindingRawType::FunctionPointer {
            abi,
            nullable,
            params,
            return_type,
        } => {
            render_shackle_binding_function_pointer_type(spec, abi, *nullable, params, return_type)
        }
    }
}

fn render_shackle_binding_function_pointer_type(
    spec: &AotInstanceProductSpec,
    abi: &str,
    nullable: bool,
    params: &[arcana_cabi::ArcanaCabiBindingRawType],
    return_type: &arcana_cabi::ArcanaCabiBindingRawType,
) -> String {
    let params = params
        .iter()
        .map(|param| render_shackle_binding_raw_type(spec, param))
        .collect::<Vec<_>>()
        .join(", ");
    let mut rendered = format!("unsafe extern {:?} fn({params})", abi);
    if !matches!(return_type, arcana_cabi::ArcanaCabiBindingRawType::Void) {
        rendered.push_str(" -> ");
        rendered.push_str(&render_shackle_binding_raw_type(spec, return_type));
    }
    if nullable {
        format!("Option<{rendered}>")
    } else {
        rendered
    }
}

fn render_shackle_binding_scalar_type(scalar: arcana_cabi::ArcanaCabiBindingScalarType) -> String {
    use arcana_cabi::ArcanaCabiBindingScalarType;

    match scalar {
        ArcanaCabiBindingScalarType::Int | ArcanaCabiBindingScalarType::I64 => "i64".to_string(),
        ArcanaCabiBindingScalarType::Bool => "bool".to_string(),
        ArcanaCabiBindingScalarType::I8 => "i8".to_string(),
        ArcanaCabiBindingScalarType::U8 => "u8".to_string(),
        ArcanaCabiBindingScalarType::I16 => "i16".to_string(),
        ArcanaCabiBindingScalarType::U16 => "u16".to_string(),
        ArcanaCabiBindingScalarType::I32 => "i32".to_string(),
        ArcanaCabiBindingScalarType::U32 => "u32".to_string(),
        ArcanaCabiBindingScalarType::U64 => "u64".to_string(),
        ArcanaCabiBindingScalarType::ISize => "isize".to_string(),
        ArcanaCabiBindingScalarType::USize => "usize".to_string(),
        ArcanaCabiBindingScalarType::F32 => "f32".to_string(),
        ArcanaCabiBindingScalarType::F64 => "f64".to_string(),
    }
}

fn render_shackle_rust_params(
    spec: &AotInstanceProductSpec,
    params: &[arcana_ir::IrRoutineParam],
) -> String {
    params
        .iter()
        .map(|param| {
            format!(
                "{}: {}",
                sanitize_identifier(&param.name),
                render_shackle_rust_type(spec, &param.ty)
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_shackle_rust_return_type(
    spec: &AotInstanceProductSpec,
    return_type: Option<&arcana_ir::IrRoutineType>,
) -> String {
    match return_type {
        Some(ty) => format!(" -> {}", render_shackle_rust_type(spec, ty)),
        None => String::new(),
    }
}

fn render_shackle_rust_type(
    spec: &AotInstanceProductSpec,
    ty: &arcana_ir::IrRoutineType,
) -> String {
    use arcana_ir::IrRoutineTypeKind;

    match &ty.kind {
        IrRoutineTypeKind::Path(path) => render_shackle_rust_path(spec, &path.segments),
        IrRoutineTypeKind::Apply { base, args } => format!(
            "{}<{}>",
            render_shackle_rust_path(spec, &base.segments),
            args.iter()
                .map(|arg| render_shackle_rust_type(spec, arg))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        IrRoutineTypeKind::Ref {
            mode,
            lifetime,
            inner,
        } => {
            let mut args = vec![render_shackle_rust_type(spec, inner)];
            if let Some(lifetime) = lifetime {
                args.push(lifetime.name.clone());
            }
            format!("&{}[{}]", mode, args.join(", "))
        }
        IrRoutineTypeKind::Tuple(items) => format!(
            "({})",
            items
                .iter()
                .map(|item| render_shackle_rust_type(spec, item))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        IrRoutineTypeKind::Projection(projection) => projection.render().replace('.', "::"),
    }
}

fn render_shackle_rust_path(spec: &AotInstanceProductSpec, segments: &[String]) -> String {
    match segments {
        [] => "()".to_string(),
        [name] if name == "Unit" => "()".to_string(),
        [name] if name == "Int" => "i64".to_string(),
        [name] if name == "Bool" => "bool".to_string(),
        [name] if name == "Str" => "String".to_string(),
        [name] if name == "c_void" => "std::ffi::c_void".to_string(),
        [name] if name == &spec.package_name => "crate".to_string(),
        [first, rest @ ..] if first == &spec.package_name => {
            format!("crate::{}", rest.join("::"))
        }
        _ => segments.join("::"),
    }
}

#[derive(Default)]
struct ShackleModuleTree<'a> {
    decls: Vec<&'a AotShackleDeclArtifact>,
    children: BTreeMap<String, ShackleModuleTree<'a>>,
}

fn build_shackle_module_tree<'a>(spec: &'a AotInstanceProductSpec) -> ShackleModuleTree<'a> {
    let mut root = ShackleModuleTree::default();
    for decl in &spec.binding_shackle_decls {
        let mut node = &mut root;
        for segment in shackle_decl_module_segments(spec, decl) {
            node = node.children.entry(segment).or_default();
        }
        node.decls.push(decl);
    }
    root
}

fn render_shackle_module_items(
    spec: &AotInstanceProductSpec,
    tree: &ShackleModuleTree<'_>,
    depth: usize,
) -> Result<String, String> {
    let mut out = String::new();
    for decl in &tree.decls {
        out.push_str(&indent_text(&render_shackle_decl_item(spec, decl)?, depth));
    }
    for (module_name, child) in &tree.children {
        let module_ident = sanitize_identifier(module_name);
        out.push_str(&indent(depth));
        out.push_str(&format!("pub(crate) mod {module_ident} {{\n"));
        out.push_str(&render_shackle_module_items(spec, child, depth + 1)?);
        out.push_str(&indent(depth));
        out.push_str("}\n\n");
    }
    Ok(out)
}

fn render_shackle_decl_item(
    spec: &AotInstanceProductSpec,
    decl: &AotShackleDeclArtifact,
) -> Result<String, String> {
    match decl.kind.as_str() {
        "fn" => {
            if let Some(binding) = decl.binding.as_deref()
                && (spec
                    .binding_imports
                    .iter()
                    .any(|import| import.name == binding)
                    || binding == "__binding.package_state_init"
                    || binding == "__binding.package_state_drop")
            {
                return Ok(String::new());
            }
            let mut out = format!(
                "#[allow(unused_variables)]\npub(crate) fn {}({}){} {{\n",
                decl.name,
                render_shackle_rust_params(spec, &decl.params),
                render_shackle_rust_return_type(spec, decl.return_type.as_ref())
            );
            out.push_str("    #[allow(unused_imports)]\n    use crate::*;\n");
            for line in &decl.body_entries {
                out.push_str("    ");
                out.push_str(line);
                out.push('\n');
            }
            out.push_str("}\n\n");
            Ok(out)
        }
        "thunk" => {
            let abi = decl
                .thunk_target
                .as_ref()
                .map(|target| target.abi.as_str())
                .unwrap_or("system");
            let mut out = format!(
                "#[allow(unused_variables)]\npub(crate) unsafe extern {:?} fn {}({}){} {{\n",
                abi,
                decl.name,
                render_shackle_rust_params(spec, &decl.params),
                render_shackle_rust_return_type(spec, decl.return_type.as_ref())
            );
            out.push_str("    #[allow(unused_imports)]\n    use crate::*;\n");
            for line in &decl.body_entries {
                out.push_str("    ");
                out.push_str(line);
                out.push('\n');
            }
            out.push_str("}\n\n");
            Ok(out)
        }
        "import fn" | "import_fn" => render_shackle_import_fn_decl(spec, decl),
        "const" => render_shackle_const_decl(spec, decl),
        "type" | "struct" | "union" | "flags" => render_shackle_raw_decl(spec, decl),
        "callback" => Ok(String::new()),
        other => Err(format!(
            "unsupported shackle declaration kind `{other}` in generated binding product"
        )),
    }
}

fn render_package_state_alias(spec: &AotInstanceProductSpec) -> Result<Option<String>, String> {
    let Some(decl) = spec.binding_shackle_decls.iter().find(|decl| {
        matches!(decl.kind.as_str(), "type" | "struct" | "union" | "flags")
            && decl.name == "PackageState"
    }) else {
        return Ok(None);
    };
    let module_path = shackle_decl_module_rust_path(spec, decl)?;
    if module_path == "crate" {
        return Ok(None);
    }
    Ok(Some(format!(
        "type PackageState = {module_path}::PackageState;\n\n"
    )))
}

fn render_package_state_init(spec: &AotInstanceProductSpec) -> Result<Option<String>, String> {
    let Some(decl) = spec.binding_shackle_decls.iter().find(|decl| {
        decl.kind == "fn" && decl.binding.as_deref() == Some("__binding.package_state_init")
    }) else {
        return Ok(None);
    };
    let mut out = String::from("fn package_state_init() -> Result<PackageState, String> {\n");
    if let Some(module_use_path) = shackle_decl_module_use_path(spec, decl) {
        out.push_str(&format!(
            "    #[allow(unused_imports)]\n    use {module_use_path}::*;\n"
        ));
    }
    for line in &decl.body_entries {
        out.push_str("    ");
        out.push_str(line);
        out.push('\n');
    }
    out.push_str("}\n\n");
    Ok(Some(out))
}

fn render_package_state_drop(spec: &AotInstanceProductSpec) -> Result<Option<String>, String> {
    let Some(decl) = spec.binding_shackle_decls.iter().find(|decl| {
        decl.kind == "fn" && decl.binding.as_deref() == Some("__binding.package_state_drop")
    }) else {
        return Ok(None);
    };
    let mut out = String::from("fn package_state_drop(state: &mut PackageState) {\n");
    if let Some(module_use_path) = shackle_decl_module_use_path(spec, decl) {
        out.push_str(&format!(
            "    #[allow(unused_imports)]\n    use {module_use_path}::*;\n"
        ));
    }
    for line in &decl.body_entries {
        out.push_str("    ");
        out.push_str(line);
        out.push('\n');
    }
    out.push_str("}\n\n");
    Ok(Some(out))
}

fn shackle_decl_module_segments(
    spec: &AotInstanceProductSpec,
    decl: &AotShackleDeclArtifact,
) -> Vec<String> {
    shackle_module_segments_for_module_id(&spec.package_name, &decl.module_id)
}

fn shackle_module_segments_for_module_id(package_name: &str, module_id: &str) -> Vec<String> {
    if module_id == package_name {
        return Vec::new();
    }
    module_id
        .strip_prefix(package_name)
        .unwrap_or(module_id)
        .trim_start_matches('.')
        .split('.')
        .filter(|segment| !segment.is_empty())
        .map(str::to_string)
        .collect()
}

fn shackle_decl_module_rust_path(
    spec: &AotInstanceProductSpec,
    decl: &AotShackleDeclArtifact,
) -> Result<String, String> {
    let segments = shackle_decl_module_segments(spec, decl);
    if segments.is_empty() {
        return Ok("crate".to_string());
    }
    Ok(format!(
        "crate::{}",
        segments
            .iter()
            .map(|segment| sanitize_identifier(segment))
            .collect::<Vec<_>>()
            .join("::")
    ))
}

fn shackle_decl_module_use_path(
    spec: &AotInstanceProductSpec,
    decl: &AotShackleDeclArtifact,
) -> Option<String> {
    shackle_decl_module_rust_path(spec, decl)
        .ok()
        .filter(|path| path != "crate")
}

fn indent(depth: usize) -> String {
    "    ".repeat(depth)
}

fn render_shackle_struct_field(line: &str) -> String {
    let trimmed = line.trim();
    if trimmed.is_empty()
        || trimmed.starts_with("pub ")
        || trimmed.starts_with("pub(")
        || trimmed.starts_with('#')
    {
        trimmed.to_string()
    } else {
        let suffix = if trimmed.ends_with(',') { "" } else { "," };
        format!("pub(crate) {trimmed}{suffix}")
    }
}

fn indent_text(text: &str, depth: usize) -> String {
    if text.is_empty() {
        return String::new();
    }
    let prefix = indent(depth);
    let mut out = String::new();
    for line in text.lines() {
        if line.is_empty() {
            out.push('\n');
        } else {
            out.push_str(&prefix);
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

fn lookup_binding_impl_decl<'a>(
    spec: &'a AotInstanceProductSpec,
    import: &NativeBindingImport,
) -> Result<&'a AotShackleDeclArtifact, String> {
    let matches = spec
        .binding_shackle_decls
        .iter()
        .filter(|decl| binding_import_matches_shackle_decl(spec, import, decl))
        .collect::<Vec<_>>();
    let decl = match matches.as_slice() {
        [decl] => *decl,
        [] => {
            return Err(format!(
                "binding import `{}` on `{}` is missing a matching shackle implementation",
                import.name, spec.package_name
            ));
        }
        _ => {
            return Err(format!(
                "binding import `{}` on `{}` has multiple matching shackle implementations",
                import.name, spec.package_name
            ));
        }
    };
    let decl_params = decl
        .params
        .iter()
        .map(parse_native_binding_param)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| {
            format!(
                "shackle impl `{}` for binding import `{}` cannot lower params: {err}",
                decl.name, import.name
            )
        })?;
    let decl_return =
        parse_native_binding_return_type(decl.return_type.as_ref()).map_err(|err| {
            format!(
                "shackle impl `{}` for binding import `{}` cannot lower return type: {err}",
                decl.name, import.name
            )
        })?;
    if decl_params != import.params || decl_return != import.return_type {
        return Err(format!(
            "shackle impl `{}` does not match binding import `{}` signature",
            decl.name, import.name
        ));
    }
    Ok(decl)
}

fn binding_import_matches_shackle_decl(
    spec: &AotInstanceProductSpec,
    import: &NativeBindingImport,
    decl: &AotShackleDeclArtifact,
) -> bool {
    if decl.kind == "fn" && decl.binding.as_deref() == Some(import.name.as_str()) {
        return true;
    }
    projected_shackle_callable_binding_name(spec, decl)
        .as_deref()
        .is_some_and(|name| name == import.name)
}

fn projected_shackle_callable_binding_name(
    spec: &AotInstanceProductSpec,
    decl: &AotShackleDeclArtifact,
) -> Option<String> {
    if !decl.exported {
        return None;
    }
    match decl.kind.as_str() {
        "fn" | "import fn" | "import_fn" | "const" => {
            let mut parts = shackle_decl_module_segments(spec, decl);
            parts.push(decl.name.clone());
            Some(parts.join("."))
        }
        _ => None,
    }
}
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root should exist")
        .to_path_buf()
}

pub fn default_instance_product_cargo_target_dir(role: ArcanaCabiProductRole) -> PathBuf {
    repo_root()
        .join("target")
        .join("arcana-cargo-targets")
        .join(format!("instance-{}", sanitize_identifier(role.as_str())))
}

fn instance_product_cargo_output_name(spec: &AotInstanceProductSpec) -> String {
    sanitize_identifier(&format!(
        "arcana_instance_{}_{}_{}",
        spec.role.as_str(),
        spec.package_id,
        spec.product_name
    ))
}

fn cargo_output_file_name(
    spec: &AotInstanceProductSpec,
    cargo_output_name: &str,
) -> Result<String, String> {
    let extension = Path::new(&spec.output_file_name)
        .extension()
        .and_then(|ext| ext.to_str())
        .filter(|ext| !ext.is_empty())
        .ok_or_else(|| {
            format!(
                "native product file `{}` is missing a valid extension",
                spec.output_file_name
            )
        })?;
    Ok(format!("{cargo_output_name}.{extension}"))
}

fn sanitize_identifier(text: &str) -> String {
    let mut out = text
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
        out.push_str("arcana_native_product");
    }
    if out.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        out.insert(0, '_');
    }
    out
}

fn render_rust_string_literal(text: &str) -> String {
    format!("{text:?}")
}

fn escape_toml(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

fn native_product_probe(event: &str, message: impl AsRef<str>) {
    if std::env::var_os(ARCANA_NATIVE_PRODUCT_TEMP_PROBES_ENV).is_some() {
        eprintln!(
            "[arcana-native-product-probe] {event}: {}",
            message.as_ref()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AotInstanceProductSpec, default_instance_product_cargo_target_dir,
        render_instance_product_cargo_toml, render_instance_product_lib_rs,
    };
    use crate::artifact::AotShackleDeclArtifact;
    use crate::native_abi::{NativeBindingCallback, NativeBindingImport};
    use arcana_cabi::{
        ARCANA_CABI_BINDING_CONTRACT_ID, ARCANA_CABI_CHILD_CONTRACT_ID,
        ARCANA_CABI_PLUGIN_CONTRACT_ID, ArcanaCabiBindingLayout,
        ArcanaCabiBindingLayoutEnumVariant, ArcanaCabiBindingLayoutField,
        ArcanaCabiBindingLayoutKind, ArcanaCabiBindingRawType, ArcanaCabiBindingScalarType,
        ArcanaCabiProductRole,
    };
    use arcana_cabi::{ArcanaCabiBindingParam, ArcanaCabiBindingType, ArcanaCabiParamSourceMode};

    fn child_spec() -> AotInstanceProductSpec {
        AotInstanceProductSpec {
            package_id: "arcana_desktop".to_string(),
            package_name: "arcana_desktop".to_string(),
            product_name: "default".to_string(),
            role: ArcanaCabiProductRole::Child,
            contract_id: ARCANA_CABI_CHILD_CONTRACT_ID.to_string(),
            output_file_name: "arcwin.dll".to_string(),
            package_image_text: None,
            binding_imports: Vec::new(),
            binding_callbacks: Vec::new(),
            binding_layouts: Vec::new(),
            binding_shackle_decls: Vec::new(),
        }
    }

    fn plugin_spec() -> AotInstanceProductSpec {
        AotInstanceProductSpec {
            package_id: "tooling".to_string(),
            package_name: "tooling".to_string(),
            product_name: "tools".to_string(),
            role: ArcanaCabiProductRole::Plugin,
            contract_id: ARCANA_CABI_PLUGIN_CONTRACT_ID.to_string(),
            output_file_name: "tooling_tools.dll".to_string(),
            package_image_text: None,
            binding_imports: Vec::new(),
            binding_callbacks: Vec::new(),
            binding_layouts: Vec::new(),
            binding_shackle_decls: Vec::new(),
        }
    }

    fn binding_spec() -> AotInstanceProductSpec {
        AotInstanceProductSpec {
            package_id: "arcana_winapi".to_string(),
            package_name: "arcana_winapi".to_string(),
            product_name: "default".to_string(),
            role: ArcanaCabiProductRole::Binding,
            contract_id: ARCANA_CABI_BINDING_CONTRACT_ID.to_string(),
            output_file_name: "arcwinapi.dll".to_string(),
            package_image_text: None,
            binding_imports: vec![NativeBindingImport {
                name: "foundation.module_path".to_string(),
                symbol_name: "arcana_binding_import_arcana_winapi_foundation_module_path"
                    .to_string(),
                return_type: ArcanaCabiBindingType::Str,
                params: vec![ArcanaCabiBindingParam::binding(
                    "module",
                    ArcanaCabiParamSourceMode::Read,
                    ArcanaCabiBindingType::Named("arcana_winapi.types.ModuleHandle".to_string()),
                )],
            }],
            binding_callbacks: vec![NativeBindingCallback {
                name: "window_proc".to_string(),
                return_type: ArcanaCabiBindingType::Int,
                params: vec![ArcanaCabiBindingParam::binding(
                    "window",
                    ArcanaCabiParamSourceMode::Edit,
                    ArcanaCabiBindingType::Named("arcana_winapi.types.HiddenWindow".to_string()),
                )],
            }],
            binding_layouts: Vec::new(),
            binding_shackle_decls: vec![AotShackleDeclArtifact {
                package_id: "arcana_winapi".to_string(),
                module_id: "arcana_winapi.foundation".to_string(),
                exported: false,
                kind: "fn".to_string(),
                name: "foundation_module_path_impl".to_string(),
                params: vec![arcana_ir::IrRoutineParam {
                    binding_id: 0,
                    mode: Some("read".to_string()),
                    name: "module".to_string(),
                    ty: arcana_ir::parse_routine_type_text("arcana_winapi.types.ModuleHandle")
                        .expect("type should parse"),
                }],
                return_type: Some(
                    arcana_ir::parse_routine_type_text("Str").expect("type should parse"),
                ),
                callback_type: None,
                binding: Some("foundation.module_path".to_string()),
                body_entries: vec!["Ok(binding_owned_str(\"module\".to_string()))".to_string()],
                raw_layout: None,
                import_target: None,
                thunk_target: None,
                surface_text: String::new(),
            }],
        }
    }

    #[test]
    fn generated_instance_product_project_uses_cdylib_and_cabi_descriptor() {
        let spec = child_spec();
        let cargo_toml =
            render_instance_product_cargo_toml(&spec).expect("cargo toml should render");
        let lib_rs = render_instance_product_lib_rs(&spec).expect("lib.rs should render");

        assert!(cargo_toml.contains("crate-type = [\"cdylib\"]"));
        assert!(cargo_toml.contains("arcana-cabi"));
        assert!(cargo_toml.contains("arcana-runtime"));
        assert!(lib_rs.contains("arcana_cabi_get_product_api_v1"));
        assert!(lib_rs.contains("ArcanaCabiChildOpsV1"));
        assert!(lib_rs.contains("run_entrypoint"));
        assert!(lib_rs.contains("\"child\\0\""));
    }

    #[test]
    fn generated_plugin_instance_product_project_exposes_use_instance() {
        let lib_rs = render_instance_product_lib_rs(&plugin_spec()).expect("lib.rs should render");
        assert!(lib_rs.contains("ArcanaCabiPluginOpsV1"));
        assert!(lib_rs.contains("describe_instance"));
        assert!(lib_rs.contains("use_instance"));
        assert!(lib_rs.contains("\"plugin\\0\""));
    }

    #[test]
    fn generated_binding_instance_product_project_exposes_self_hosted_binding_ops() {
        let cargo_toml =
            render_instance_product_cargo_toml(&binding_spec()).expect("cargo toml should render");
        let lib_rs = render_instance_product_lib_rs(&binding_spec()).expect("lib.rs should render");

        assert!(cargo_toml.contains("crate-type = [\"cdylib\"]"));
        assert!(cargo_toml.contains("arcana-cabi"));
        assert!(!cargo_toml.contains("arcana-runtime"));
        assert!(lib_rs.contains("ArcanaCabiBindingOpsV1"));
        assert!(lib_rs.contains("RegisteredCallback"));
        assert!(lib_rs.contains("run_binding_import"));
        assert!(lib_rs.contains("binding_import_impl_0"));
        assert!(lib_rs.contains("arcana_binding_import_arcana_winapi_foundation_module_path"));
        assert!(lib_rs.contains("binding_callback_name_is_declared"));
        assert!(lib_rs.contains("is not declared by this product"));
        assert!(lib_rs.contains("is already registered"));
        assert!(lib_rs.contains(
            "release_binding_output_value(out, callback.owned_bytes_free, callback.owned_str_free)"
        ));
        assert!(lib_rs.contains("\"binding\\0\""));
    }

    #[test]
    fn generated_binding_instance_product_rewrites_package_qualified_shackle_type_aliases() {
        let mut spec = binding_spec();
        spec.binding_shackle_decls.push(AotShackleDeclArtifact {
            package_id: "arcana_winapi".to_string(),
            module_id: "arcana_winapi.raw.types".to_string(),
            exported: true,
            kind: "type".to_string(),
            name: "PHMODULE".to_string(),
            params: Vec::new(),
            return_type: None,
            callback_type: None,
            binding: Some("*mut arcana_winapi.raw.types.HMODULE".to_string()),
            body_entries: Vec::new(),
            raw_layout: None,
            import_target: None,
            thunk_target: None,
            surface_text: String::new(),
        });
        let lib_rs = render_instance_product_lib_rs(&spec).expect("lib.rs should render");
        assert!(lib_rs.contains("pub(crate) mod raw"));
        assert!(lib_rs.contains("pub(crate) mod types"));
        assert!(lib_rs.contains("pub(crate) type PHMODULE = *mut crate::raw::types::HMODULE;"));
    }

    #[test]
    fn generated_binding_instance_product_renders_typed_raw_enums_and_enum_const_bindings() {
        let mut spec = binding_spec();
        let mut decls = spec.binding_shackle_decls.clone();
        decls.extend([
            AotShackleDeclArtifact {
                package_id: "arcana_winapi".to_string(),
                module_id: "arcana_winapi.raw.types".to_string(),
                exported: true,
                kind: "type".to_string(),
                name: "DWRITE_FACTORY_TYPE".to_string(),
                params: Vec::new(),
                return_type: None,
                callback_type: None,
                binding: Some("U32".to_string()),
                body_entries: vec!["Shared = 0".to_string(), "Isolated = 1".to_string()],
                raw_layout: Some(ArcanaCabiBindingLayout {
                    layout_id: "arcana_winapi.raw.types.DWRITE_FACTORY_TYPE".to_string(),
                    size: 4,
                    align: 4,
                    kind: ArcanaCabiBindingLayoutKind::Enum {
                        repr: ArcanaCabiBindingScalarType::U32,
                        variants: vec![
                            ArcanaCabiBindingLayoutEnumVariant {
                                name: "Shared".to_string(),
                                value: 0,
                            },
                            ArcanaCabiBindingLayoutEnumVariant {
                                name: "Isolated".to_string(),
                                value: 1,
                            },
                        ],
                    },
                }),
                import_target: None,
                thunk_target: None,
                surface_text: String::new(),
            },
            AotShackleDeclArtifact {
                package_id: "arcana_winapi".to_string(),
                module_id: "arcana_winapi.raw.constants".to_string(),
                exported: true,
                kind: "const".to_string(),
                name: "DWRITE_FACTORY_TYPE_SHARED".to_string(),
                params: Vec::new(),
                return_type: Some(
                    arcana_ir::parse_routine_type_text(
                        "arcana_winapi.raw.types.DWRITE_FACTORY_TYPE",
                    )
                    .expect("type should parse"),
                ),
                callback_type: None,
                binding: Some("arcana_winapi.raw.types.DWRITE_FACTORY_TYPE.Shared".to_string()),
                body_entries: Vec::new(),
                raw_layout: None,
                import_target: None,
                thunk_target: None,
                surface_text: String::new(),
            },
        ]);
        spec.binding_shackle_decls = decls;

        let lib_rs = render_instance_product_lib_rs(&spec).expect("lib.rs should render");

        assert!(lib_rs.contains("pub(crate) type DWRITE_FACTORY_TYPE = u32;"));
        assert!(lib_rs.contains("pub(crate) struct DWRITE_FACTORY_TYPE__ValueSet"));
        assert!(
            lib_rs.contains("pub(crate) const DWRITE_FACTORY_TYPE: DWRITE_FACTORY_TYPE__ValueSet")
        );
        assert!(lib_rs.contains("Shared: 0 as DWRITE_FACTORY_TYPE"));
        assert!(lib_rs.contains(
            "pub(crate) const DWRITE_FACTORY_TYPE_SHARED: crate::raw::types::DWRITE_FACTORY_TYPE = crate::raw::types::DWRITE_FACTORY_TYPE.Shared;"
        ));
    }

    #[test]
    fn generated_binding_instance_product_emits_typed_raw_layout_tables() {
        let mut spec = binding_spec();
        spec.binding_layouts = vec![ArcanaCabiBindingLayout {
            layout_id: "arcana_winapi.raw.Rect".to_string(),
            size: 12,
            align: 4,
            kind: ArcanaCabiBindingLayoutKind::Struct {
                fields: vec![
                    ArcanaCabiBindingLayoutField {
                        name: "left".to_string(),
                        ty: ArcanaCabiBindingRawType::Scalar(ArcanaCabiBindingScalarType::I32),
                        offset: 0,
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
        }];

        let lib_rs = render_instance_product_lib_rs(&spec).expect("lib.rs should render");

        assert!(lib_rs.contains("ArcanaCabiBindingLayoutEntryV1"));
        assert!(
            lib_rs.contains(
                "static BINDING_LAYOUTS: [arcana_cabi::ArcanaCabiBindingLayoutEntryV1; 1]"
            )
        );
        assert!(lib_rs.contains("static BINDING_LAYOUT_0_DETAIL_JSON"));
        assert!(lib_rs.contains("arcana_winapi.raw.Rect"));
        assert!(
            lib_rs.contains("detail_json: BINDING_LAYOUT_0_DETAIL_JSON.as_ptr() as *const c_char")
        );
        assert!(lib_rs.contains("layouts: BINDING_LAYOUTS.as_ptr()"));
        assert!(lib_rs.contains("layout_count: BINDING_LAYOUTS.len()"));
    }

    #[test]
    fn generated_binding_instance_product_projects_exported_shackle_import_fns_and_consts() {
        let mut spec = binding_spec();
        spec.binding_imports = vec![
            NativeBindingImport {
                name: "raw.kernel32.GetCurrentProcessId".to_string(),
                symbol_name: "arcana_binding_import_arcana_winapi_raw_kernel32_getcurrentprocessid"
                    .to_string(),
                return_type: ArcanaCabiBindingType::Int,
                params: Vec::new(),
            },
            NativeBindingImport {
                name: "raw.constants.MAGIC".to_string(),
                symbol_name: "arcana_binding_import_arcana_winapi_raw_constants_magic".to_string(),
                return_type: ArcanaCabiBindingType::Int,
                params: Vec::new(),
            },
        ];
        spec.binding_shackle_decls = vec![
            AotShackleDeclArtifact {
                package_id: "arcana_winapi".to_string(),
                module_id: "arcana_winapi.raw.kernel32".to_string(),
                exported: true,
                kind: "import_fn".to_string(),
                name: "GetCurrentProcessId".to_string(),
                params: Vec::new(),
                return_type: Some(
                    arcana_ir::parse_routine_type_text("Int").expect("type should parse"),
                ),
                callback_type: None,
                binding: Some("kernel32.GetCurrentProcessId".to_string()),
                body_entries: Vec::new(),
                raw_layout: None,
                import_target: Some(crate::artifact::AotShackleImportTargetArtifact {
                    library: "kernel32".to_string(),
                    symbol: "GetCurrentProcessId".to_string(),
                    abi: "system".to_string(),
                }),
                thunk_target: None,
                surface_text: String::new(),
            },
            AotShackleDeclArtifact {
                package_id: "arcana_winapi".to_string(),
                module_id: "arcana_winapi.raw.constants".to_string(),
                exported: true,
                kind: "const".to_string(),
                name: "MAGIC".to_string(),
                params: Vec::new(),
                return_type: Some(
                    arcana_ir::parse_routine_type_text("Int").expect("type should parse"),
                ),
                callback_type: None,
                binding: Some("7".to_string()),
                body_entries: Vec::new(),
                raw_layout: None,
                import_target: None,
                thunk_target: None,
                surface_text: String::new(),
            },
        ];

        let lib_rs = render_instance_product_lib_rs(&spec).expect("lib.rs should render");

        assert!(lib_rs.contains("pub fn GetCurrentProcessId() -> i64;"));
        assert!(lib_rs.contains("Ok(binding_int(unsafe { GetCurrentProcessId() } as i64))"));
        assert!(lib_rs.contains("pub(crate) const MAGIC: i64 = 7;"));
        assert!(lib_rs.contains("Ok(binding_int(MAGIC as i64))"));
    }

    #[test]
    fn default_instance_product_cargo_target_dir_is_stable_for_role() {
        let first = default_instance_product_cargo_target_dir(ArcanaCabiProductRole::Child);
        let second = default_instance_product_cargo_target_dir(ArcanaCabiProductRole::Child);
        assert_eq!(first, second);
        assert!(
            first
                .ends_with(std::path::PathBuf::from("arcana-cargo-targets").join("instance-child")),
            "shared instance-product cargo target dir should stay under target/arcana-cargo-targets"
        );
    }
}
