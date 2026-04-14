use std::path::Path;

// Bundle metadata projection of the cabi export contract, not a second ABI owner.
use arcana_cabi::{
    ARCANA_CABI_GET_PRODUCT_API_V1_SYMBOL, ARCANA_CABI_LAST_ERROR_ALLOC_V1_SYMBOL,
    ARCANA_CABI_OWNED_BYTES_FREE_V1_SYMBOL, ARCANA_CABI_OWNED_STR_FREE_V1_SYMBOL,
};
use serde::{Deserialize, Serialize};

use crate::native_abi::{
    NativeAbiType, NativeBindingCallback, NativeBindingImport, NativeExport,
    collect_native_binding_callbacks, collect_native_binding_imports,
};
use crate::native_plan::{NativeLaunchPlan, NativePackagePlan};

pub const NATIVE_BUNDLE_MANIFEST_FORMAT: &str = "arcana-native-manifest-v3";
const EMBEDDED_RUNTIME_PAYLOAD_KIND: &str = "embedded-runtime-package-image";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeBundleManifest {
    pub format: String,
    pub target: String,
    pub target_format: String,
    pub package_name: String,
    pub root_module_id: String,
    pub root_artifact: String,
    pub payload_kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contract_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contract_version: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding: Option<NativeBundleBindingManifest>,
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
    #[serde(alias = "bytes_free_symbol")]
    pub owned_bytes_free_symbol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owned_str_free_symbol: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub exports: Vec<NativeBundleExportManifest>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeBundleExportManifest {
    pub export_name: String,
    pub routine_key: String,
    pub symbol_name: String,
    pub return_type: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub params: Vec<NativeBundleParamManifest>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeBundleParamManifest {
    pub name: String,
    pub source_mode: String,
    pub pass_mode: String,
    #[serde(alias = "ty")]
    pub input_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(alias = "write_back_ty")]
    pub write_back_type: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeBundleBindingManifest {
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub imports: Vec<NativeBundleBindingImportManifest>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub callbacks: Vec<NativeBundleBindingCallbackManifest>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub layouts: Vec<NativeBundleBindingLayoutManifest>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeBundleBindingImportManifest {
    pub name: String,
    pub symbol_name: String,
    pub return_type: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub params: Vec<NativeBundleParamManifest>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeBundleBindingCallbackManifest {
    pub name: String,
    pub return_type: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub params: Vec<NativeBundleParamManifest>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeBundleBindingLayoutManifest {
    pub layout_id: String,
    pub detail_json: String,
}

pub fn render_native_bundle_manifest(plan: &NativePackagePlan) -> Result<String, String> {
    toml::to_string(&native_bundle_manifest_for_plan(plan)?)
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
    out.push_str(&format!("    {ARCANA_CABI_GET_PRODUCT_API_V1_SYMBOL}\n"));
    out.push_str(&format!("    {ARCANA_CABI_LAST_ERROR_ALLOC_V1_SYMBOL}\n"));
    out.push_str(&format!("    {ARCANA_CABI_OWNED_BYTES_FREE_V1_SYMBOL}\n"));
    out.push_str(&format!("    {ARCANA_CABI_OWNED_STR_FREE_V1_SYMBOL}\n"));
    for export in exports {
        out.push_str("    ");
        out.push_str(&export.export_name);
        out.push('\n');
    }
    Ok(out)
}

fn native_bundle_manifest_for_plan(
    plan: &NativePackagePlan,
) -> Result<NativeBundleManifest, String> {
    Ok(NativeBundleManifest {
        format: NATIVE_BUNDLE_MANIFEST_FORMAT.to_string(),
        target: target_key(plan).to_string(),
        target_format: plan.target.format().to_string(),
        package_name: plan.artifact.package_name.clone(),
        root_module_id: plan.artifact.root_module_id.clone(),
        root_artifact: plan.root_artifact_file_name.clone(),
        payload_kind: EMBEDDED_RUNTIME_PAYLOAD_KIND.to_string(),
        product_name: plan
            .native_product
            .as_ref()
            .map(|product| product.name.clone()),
        product_role: plan
            .native_product
            .as_ref()
            .map(|product| product.role.as_str().to_string()),
        contract_id: plan
            .native_product
            .as_ref()
            .map(|product| product.contract_id.clone()),
        contract_version: plan
            .native_product
            .as_ref()
            .map(|product| product.contract_version),
        binding: native_binding_manifest_for_plan(plan)?,
        launch: native_launch_manifest_for_plan(plan),
    })
}

fn native_binding_manifest_for_plan(
    plan: &NativePackagePlan,
) -> Result<Option<NativeBundleBindingManifest>, String> {
    let is_binding_product = plan
        .native_product
        .as_ref()
        .is_some_and(|product| product.role.as_str() == "binding");
    if !is_binding_product {
        return Ok(None);
    }
    let imports = collect_native_binding_imports(&plan.artifact)?
        .into_iter()
        .map(native_binding_import_manifest)
        .collect::<Vec<_>>();
    let callbacks = collect_native_binding_callbacks(&plan.artifact)?
        .into_iter()
        .map(native_binding_callback_manifest)
        .collect::<Vec<_>>();
    let layouts = plan
        .artifact
        .binding_layouts
        .iter()
        .map(|layout| {
            serde_json::to_string(layout)
                .map(|detail_json| NativeBundleBindingLayoutManifest {
                    layout_id: layout.layout_id.clone(),
                    detail_json,
                })
                .map_err(|e| {
                    format!(
                        "failed to serialize binding layout `{}` into native manifest: {e}",
                        layout.layout_id
                    )
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Some(NativeBundleBindingManifest {
        imports,
        callbacks,
        layouts,
    }))
}

fn native_launch_manifest_for_plan(plan: &NativePackagePlan) -> NativeBundleLaunchManifest {
    match &plan.launch {
        NativeLaunchPlan::Executable { main_routine_key } => NativeBundleLaunchManifest {
            kind: "executable".to_string(),
            main_routine_key: Some(main_routine_key.clone()),
            header: None,
            definition_file: None,
            last_error_alloc_symbol: None,
            owned_bytes_free_symbol: None,
            owned_str_free_symbol: None,
            exports: Vec::new(),
        },
        NativeLaunchPlan::DynamicLibrary { exports } => NativeBundleLaunchManifest {
            kind: "dynamic-library".to_string(),
            main_routine_key: None,
            header: Some(windows_dll_header_file_name(&plan.root_artifact_file_name)),
            definition_file: Some(windows_dll_definition_file_name(
                &plan.root_artifact_file_name,
            )),
            last_error_alloc_symbol: Some(ARCANA_CABI_LAST_ERROR_ALLOC_V1_SYMBOL.to_string()),
            owned_bytes_free_symbol: Some(ARCANA_CABI_OWNED_BYTES_FREE_V1_SYMBOL.to_string()),
            owned_str_free_symbol: Some(ARCANA_CABI_OWNED_STR_FREE_V1_SYMBOL.to_string()),
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
        symbol_name: export.symbol_name.clone(),
        return_type: native_type_name(&export.return_type),
        params: export
            .params
            .iter()
            .map(|param| NativeBundleParamManifest {
                name: param.name.clone(),
                source_mode: param.source_mode.as_str().to_string(),
                pass_mode: param.pass_mode.as_str().to_string(),
                input_type: native_type_name(&param.input_type),
                write_back_type: param.write_back_type.as_ref().map(native_type_name),
            })
            .collect(),
    }
}

fn native_binding_import_manifest(
    import: NativeBindingImport,
) -> NativeBundleBindingImportManifest {
    NativeBundleBindingImportManifest {
        name: import.name,
        symbol_name: import.symbol_name,
        return_type: import.return_type.render(),
        params: import
            .params
            .iter()
            .map(native_binding_param_manifest)
            .collect(),
    }
}

fn native_binding_callback_manifest(
    callback: NativeBindingCallback,
) -> NativeBundleBindingCallbackManifest {
    NativeBundleBindingCallbackManifest {
        name: callback.name,
        return_type: callback.return_type.render(),
        params: callback
            .params
            .iter()
            .map(native_binding_param_manifest)
            .collect(),
    }
}

fn native_binding_param_manifest(
    param: &arcana_cabi::ArcanaCabiBindingParam,
) -> NativeBundleParamManifest {
    NativeBundleParamManifest {
        name: param.name.clone(),
        source_mode: param.source_mode.as_str().to_string(),
        pass_mode: param.pass_mode.as_str().to_string(),
        input_type: param.input_type.render(),
        write_back_type: param.write_back_type.as_ref().map(|ty| ty.render()),
    }
}

fn native_type_name(ty: &NativeAbiType) -> String {
    match ty {
        NativeAbiType::Int => "Int".to_string(),
        NativeAbiType::Bool => "Bool".to_string(),
        NativeAbiType::Str => "Str".to_string(),
        NativeAbiType::Bytes => "Bytes".to_string(),
        NativeAbiType::Opaque(name) => name.clone(),
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
