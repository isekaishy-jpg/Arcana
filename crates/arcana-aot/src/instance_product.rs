use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use arcana_cabi::ArcanaCabiProductRole;

pub const ARCANA_NATIVE_PRODUCT_TEMP_PROBES_ENV: &str = "ARCANA_NATIVE_PRODUCT_TEMP_PROBES";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AotInstanceProductSpec {
    pub package_name: String,
    pub product_name: String,
    pub role: ArcanaCabiProductRole,
    pub contract_id: String,
    pub output_file_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AotCompiledInstanceProduct {
    pub output_path: PathBuf,
}

pub fn compile_instance_product(
    spec: &AotInstanceProductSpec,
    project_dir: &Path,
    target_dir: &Path,
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
            "package={} product={} role={} contract={} project_dir={} target_dir={}",
            spec.package_name,
            spec.product_name,
            spec.role.as_str(),
            spec.contract_id,
            project_dir.display(),
            target_dir.display()
        ),
    );

    write_instance_product_project(project_dir, spec)?;

    fs::create_dir_all(target_dir).map_err(|e| {
        format!(
            "failed to create native product target directory `{}`: {e}",
            target_dir.display()
        )
    })?;

    let manifest_path = project_dir.join("Cargo.toml");
    let status = Command::new("cargo")
        .arg("build")
        .arg("-q")
        .arg("--manifest-path")
        .arg(&manifest_path)
        .arg("--target-dir")
        .arg(target_dir)
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

    let output_path = target_dir.join("debug").join(&spec.output_file_name);
    if !output_path.is_file() {
        native_product_probe(
            "compile_missing_output",
            format!(
                "package={} product={} expected_output={}",
                spec.package_name,
                spec.product_name,
                output_path.display()
            ),
        );
        return Err(format!(
            "generated native product `{}` on `{}` did not produce `{}` under `{}`",
            spec.product_name,
            spec.package_name,
            spec.output_file_name,
            target_dir.join("debug").display()
        ));
    }

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
    spec: &AotInstanceProductSpec,
) -> Result<(), String> {
    if project_dir.exists() {
        fs::remove_dir_all(project_dir).map_err(|e| {
            format!(
                "failed to clear generated native product project `{}`: {e}",
                project_dir.display()
            )
        })?;
    }
    fs::create_dir_all(project_dir.join("src")).map_err(|e| {
        format!(
            "failed to create generated native product project `{}`: {e}",
            project_dir.display()
        )
    })?;
    fs::write(
        project_dir.join("Cargo.toml"),
        render_instance_product_cargo_toml(spec)?,
    )
    .map_err(|e| {
        format!(
            "failed to write generated native product Cargo.toml `{}`: {e}",
            project_dir.join("Cargo.toml").display()
        )
    })?;
    fs::write(
        project_dir.join("src").join("lib.rs"),
        render_instance_product_lib_rs(spec),
    )
    .map_err(|e| {
        format!(
            "failed to write generated native product lib.rs `{}`: {e}",
            project_dir.join("src").join("lib.rs").display()
        )
    })?;
    Ok(())
}

fn render_instance_product_cargo_toml(spec: &AotInstanceProductSpec) -> Result<String, String> {
    let repo_root = repo_root();
    let cabi_dependency = repo_root.join("crates").join("arcana-cabi");
    let runtime_dependency = repo_root.join("crates").join("arcana-runtime");
    let output_stem = output_stem(&spec.output_file_name)?;
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
            sanitize_identifier(&spec.package_name),
            sanitize_identifier(&spec.product_name)
        )),
        escape_toml(&output_stem),
        escape_toml(&cabi_dependency.display().to_string()),
    );
    if spec.role == ArcanaCabiProductRole::Child {
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
            "unsafe extern \"system\" fn create_unit_instance() -> *mut c_void {{\n",
            "    Box::into_raw(Box::new(())) as *mut c_void\n",
            "}}\n\n",
            "unsafe extern \"system\" fn destroy_unit_instance(instance: *mut c_void) {{\n",
            "    if instance.is_null() {{\n",
            "        return;\n",
            "    }}\n",
            "    unsafe {{\n",
            "        drop(Box::from_raw(instance as *mut ()));\n",
            "    }}\n",
            "}}\n\n",
        ),
        render_rust_string_literal(&package_name),
        render_rust_string_literal(&product_name),
        render_rust_string_literal(&role),
        render_rust_string_literal(&contract),
    )
}

fn render_child_instance_product_lib_rs(spec: &AotInstanceProductSpec) -> String {
    let mut out = render_common_instance_preamble(spec);
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
    out.push_str(&format!(
        "static PLUGIN_DESCRIPTION: &str = {};\n\n",
        render_rust_string_literal(&description)
    ));
    out.push_str(
        concat!(
            "use arcana_cabi::{ArcanaCabiInstanceOpsV1, ArcanaCabiPluginOpsV1};\n\n",
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
            "static PLUGIN_OPS: ArcanaCabiPluginOpsV1 = ArcanaCabiPluginOpsV1 {\n",
            "    base: ArcanaCabiInstanceOpsV1 {\n",
            "        ops_size: std::mem::size_of::<ArcanaCabiInstanceOpsV1>(),\n",
            "        create_instance: create_unit_instance as ArcanaCabiCreateInstanceFn,\n",
            "        destroy_instance: destroy_unit_instance as ArcanaCabiDestroyInstanceFn,\n",
            "        reserved0: ptr::null(),\n",
            "        reserved1: ptr::null(),\n",
            "    },\n",
            "    describe_instance,\n",
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

fn output_stem(file_name: &str) -> Result<String, String> {
    Path::new(file_name)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("native product file `{file_name}` is missing a valid file stem"))
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
        AotInstanceProductSpec, render_instance_product_cargo_toml, render_instance_product_lib_rs,
    };
    use arcana_cabi::{ARCANA_CABI_CHILD_CONTRACT_ID, ArcanaCabiProductRole};

    fn child_spec() -> AotInstanceProductSpec {
        AotInstanceProductSpec {
            package_name: "arcana_desktop".to_string(),
            product_name: "default".to_string(),
            role: ArcanaCabiProductRole::Child,
            contract_id: ARCANA_CABI_CHILD_CONTRACT_ID.to_string(),
            output_file_name: "arcana_desktop.dll".to_string(),
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
}
