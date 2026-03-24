use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::native_abi::{NativeAbiType, NativeExport};
use crate::native_plan::{NativeLaunchPlan, NativePackagePlan};

pub const NATIVE_BUNDLE_MANIFEST_FORMAT: &str = "arcana-native-manifest-v2";
const EMBEDDED_RUNTIME_PAYLOAD_KIND: &str = "embedded-runtime-package-image";
const DLL_LAST_ERROR_ALLOC_SYMBOL: &str = "arcana_last_error_alloc";
const DLL_BYTES_FREE_SYMBOL: &str = "arcana_bytes_free";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeBundleManifest {
    pub format: String,
    pub target: String,
    pub target_format: String,
    pub package_name: String,
    pub root_module_id: String,
    pub root_artifact: String,
    pub payload_kind: String,
    pub launch: NativeBundleLaunchManifest,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeBundleLaunchManifest {
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub main_routine_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definition_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error_alloc_symbol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes_free_symbol: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub exports: Vec<NativeBundleExportManifest>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeBundleExportManifest {
    pub export_name: String,
    pub routine_key: String,
    pub return_type: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub params: Vec<NativeBundleParamManifest>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeBundleParamManifest {
    pub name: String,
    pub ty: String,
}

pub fn render_native_bundle_manifest(plan: &NativePackagePlan) -> Result<String, String> {
    toml::to_string(&native_bundle_manifest_for_plan(plan))
        .map_err(|e| format!("failed to render native bundle manifest: {e}"))
}

pub fn parse_native_bundle_manifest(text: &str) -> Result<NativeBundleManifest, String> {
    let manifest = toml::from_str::<NativeBundleManifest>(text)
        .map_err(|e| format!("failed to parse native bundle manifest: {e}"))?;
    if manifest.format != NATIVE_BUNDLE_MANIFEST_FORMAT {
        return Err(format!(
            "unsupported native bundle manifest format `{}`; expected `{NATIVE_BUNDLE_MANIFEST_FORMAT}`",
            manifest.format
        ));
    }
    Ok(manifest)
}

pub fn native_bundle_manifest_file_name(root_artifact_file_name: &str) -> String {
    format!("{root_artifact_file_name}.arcana-bundle.toml")
}

pub fn windows_dll_header_file_name(root_artifact_file_name: &str) -> String {
    format!("{root_artifact_file_name}.h")
}

pub fn windows_dll_definition_file_name(root_artifact_file_name: &str) -> String {
    format!("{root_artifact_file_name}.def")
}

pub fn render_windows_dll_definition_file(plan: &NativePackagePlan) -> Result<String, String> {
    let NativeLaunchPlan::DynamicLibrary { exports } = &plan.launch else {
        return Err(
            "windows dll definition file requires a dynamic-library native plan".to_string(),
        );
    };
    let library_name = Path::new(&plan.root_artifact_file_name)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or("arcana");
    let mut out = format!("LIBRARY \"{library_name}\"\nEXPORTS\n");
    out.push_str(&format!("    {DLL_LAST_ERROR_ALLOC_SYMBOL}\n"));
    out.push_str(&format!("    {DLL_BYTES_FREE_SYMBOL}\n"));
    for export in exports {
        out.push_str("    ");
        out.push_str(&export.export_name);
        out.push('\n');
    }
    Ok(out)
}

fn native_bundle_manifest_for_plan(plan: &NativePackagePlan) -> NativeBundleManifest {
    NativeBundleManifest {
        format: NATIVE_BUNDLE_MANIFEST_FORMAT.to_string(),
        target: target_key(plan).to_string(),
        target_format: plan.target.format().to_string(),
        package_name: plan.artifact.package_name.clone(),
        root_module_id: plan.artifact.root_module_id.clone(),
        root_artifact: plan.root_artifact_file_name.clone(),
        payload_kind: EMBEDDED_RUNTIME_PAYLOAD_KIND.to_string(),
        launch: native_launch_manifest_for_plan(plan),
    }
}

fn native_launch_manifest_for_plan(plan: &NativePackagePlan) -> NativeBundleLaunchManifest {
    match &plan.launch {
        NativeLaunchPlan::Executable { main_routine_key } => NativeBundleLaunchManifest {
            kind: "executable".to_string(),
            main_routine_key: Some(main_routine_key.clone()),
            header: None,
            definition_file: None,
            last_error_alloc_symbol: None,
            bytes_free_symbol: None,
            exports: Vec::new(),
        },
        NativeLaunchPlan::DynamicLibrary { exports } => NativeBundleLaunchManifest {
            kind: "dynamic-library".to_string(),
            main_routine_key: None,
            header: Some(windows_dll_header_file_name(&plan.root_artifact_file_name)),
            definition_file: Some(windows_dll_definition_file_name(
                &plan.root_artifact_file_name,
            )),
            last_error_alloc_symbol: Some(DLL_LAST_ERROR_ALLOC_SYMBOL.to_string()),
            bytes_free_symbol: Some(DLL_BYTES_FREE_SYMBOL.to_string()),
            exports: exports
                .iter()
                .map(native_export_manifest)
                .collect::<Vec<_>>(),
        },
    }
}

fn native_export_manifest(export: &NativeExport) -> NativeBundleExportManifest {
    NativeBundleExportManifest {
        export_name: export.export_name.clone(),
        routine_key: export.routine_key.clone(),
        return_type: native_type_name(&export.return_type),
        params: export
            .params
            .iter()
            .map(|param| NativeBundleParamManifest {
                name: param.name.clone(),
                ty: native_type_name(&param.ty),
            })
            .collect(),
    }
}

fn native_type_name(ty: &NativeAbiType) -> String {
    match ty {
        NativeAbiType::Int => "Int".to_string(),
        NativeAbiType::Bool => "Bool".to_string(),
        NativeAbiType::Str => "Str".to_string(),
        NativeAbiType::Bytes => "Array[Int]".to_string(),
        NativeAbiType::Unit => "Unit".to_string(),
        NativeAbiType::Pair(left, right) => {
            format!(
                "Pair[{}, {}]",
                native_type_name(left),
                native_type_name(right)
            )
        }
    }
}

fn target_key(plan: &NativePackagePlan) -> &'static str {
    match plan.target {
        crate::emit::AotEmitTarget::InternalArtifact => "internal-aot",
        crate::emit::AotEmitTarget::WindowsExeBundle => "windows-exe",
        crate::emit::AotEmitTarget::WindowsDllBundle => "windows-dll",
    }
}
