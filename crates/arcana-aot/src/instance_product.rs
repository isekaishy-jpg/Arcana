use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::Command;

use arcana_cabi::ArcanaCabiProductRole;
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
        ArcanaCabiProductRole::Child | ArcanaCabiProductRole::Plugin
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
            "generic native instance products support only `child` and `plugin` roles (found `{}` for `{}:{}`)",
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
    let lib_rs = render_instance_product_lib_rs(spec);
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

fn render_instance_product_lib_rs(spec: &AotInstanceProductSpec) -> String {
    match spec.role {
        ArcanaCabiProductRole::Child => render_child_instance_product_lib_rs(spec),
        ArcanaCabiProductRole::Plugin => render_plugin_instance_product_lib_rs(spec),
        ArcanaCabiProductRole::Export => unreachable!("instance products reject export role"),
        ArcanaCabiProductRole::Binding => unreachable!("instance products reject binding role"),
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
    use arcana_cabi::{
        ARCANA_CABI_CHILD_CONTRACT_ID, ARCANA_CABI_PLUGIN_CONTRACT_ID, ArcanaCabiProductRole,
    };

    fn child_spec() -> AotInstanceProductSpec {
        AotInstanceProductSpec {
            package_id: "arcana_desktop".to_string(),
            package_name: "arcana_desktop".to_string(),
            product_name: "default".to_string(),
            role: ArcanaCabiProductRole::Child,
            contract_id: ARCANA_CABI_CHILD_CONTRACT_ID.to_string(),
            output_file_name: "arcwin.dll".to_string(),
            package_image_text: None,
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
        }
    }

    #[test]
    fn generated_instance_product_project_uses_cdylib_and_cabi_descriptor() {
        let spec = child_spec();
        let cargo_toml =
            render_instance_product_cargo_toml(&spec).expect("cargo toml should render");
        let lib_rs = render_instance_product_lib_rs(&spec);

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
        let lib_rs = render_instance_product_lib_rs(&plugin_spec());
        assert!(lib_rs.contains("ArcanaCabiPluginOpsV1"));
        assert!(lib_rs.contains("describe_instance"));
        assert!(lib_rs.contains("use_instance"));
        assert!(lib_rs.contains("\"plugin\\0\""));
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
