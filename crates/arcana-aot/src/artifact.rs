use arcana_cabi::{
    ArcanaCabiApiBackendTargetKind, ArcanaCabiApiFieldContract, ArcanaCabiBindingLayout,
};
use arcana_ir::{
    ExecAvailabilityAttachment, ExecCleanupFooter, ExecExpr, ExecStmt, IrForewordMetadata,
    IrForewordRegistrationRow, IrRoutineParam, IrRoutineType,
};
use serde::{Deserialize, Serialize};

pub const AOT_INTERNAL_FORMAT: &str = "arcana-aot-v9";

pub type AotRoutineParamArtifact = IrRoutineParam;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AotArtifact {
    pub format: String,
    pub symbol_count: usize,
    pub item_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AotPackageModuleArtifact {
    pub package_id: String,
    pub module_id: String,
    pub symbol_count: usize,
    pub item_count: usize,
    pub line_count: usize,
    pub non_empty_line_count: usize,
    pub directive_rows: Vec<String>,
    pub lang_item_rows: Vec<String>,
    pub exported_surface_rows: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AotEntrypointArtifact {
    pub package_id: String,
    pub module_id: String,
    pub symbol_name: String,
    pub symbol_kind: String,
    pub is_async: bool,
    pub exported: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AotRoutineArtifact {
    pub package_id: String,
    pub module_id: String,
    pub routine_key: String,
    pub symbol_name: String,
    pub symbol_kind: String,
    pub exported: bool,
    pub is_async: bool,
    pub type_params: Vec<String>,
    pub behavior_attrs: std::collections::BTreeMap<String, String>,
    pub params: Vec<IrRoutineParam>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_type: Option<IrRoutineType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intrinsic_impl: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_impl: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub impl_target_type: Option<IrRoutineType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub impl_trait_path: Option<Vec<String>>,
    pub availability: Vec<ExecAvailabilityAttachment>,
    pub cleanup_footers: Vec<ExecCleanupFooter>,
    pub statements: Vec<ExecStmt>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AotNativeCallbackArtifact {
    pub package_id: String,
    pub module_id: String,
    pub name: String,
    pub params: Vec<IrRoutineParam>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_type: Option<IrRoutineType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub callback_type: Option<IrRoutineType>,
    pub target: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_routine_key: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AotShackleImportTargetArtifact {
    pub library: String,
    pub symbol: String,
    pub abi: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AotShackleThunkTargetArtifact {
    pub target: String,
    pub abi: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AotShackleDeclArtifact {
    pub package_id: String,
    pub module_id: String,
    pub exported: bool,
    pub kind: String,
    pub name: String,
    pub params: Vec<IrRoutineParam>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_type: Option<IrRoutineType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub callback_type: Option<IrRoutineType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binding: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub body_entries: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_layout: Option<ArcanaCabiBindingLayout>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub import_target: Option<AotShackleImportTargetArtifact>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thunk_target: Option<AotShackleThunkTargetArtifact>,
    pub surface_text: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AotApiDeclArtifact {
    pub package_id: String,
    pub module_id: String,
    pub exported: bool,
    pub name: String,
    pub request_type: IrRoutineType,
    pub response_type: IrRoutineType,
    pub backend_target_kind: ArcanaCabiApiBackendTargetKind,
    pub backend_target: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<ArcanaCabiApiFieldContract>,
    pub surface_text: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AotOwnerObjectArtifact {
    pub type_path: Vec<String>,
    pub local_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub init_routine_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub init_with_context_routine_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resume_routine_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resume_with_context_routine_key: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AotOwnerExitArtifact {
    pub name: String,
    pub condition: ExecExpr,
    pub retains: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AotOwnerArtifact {
    #[serde(default)]
    pub package_id: String,
    pub module_id: String,
    pub owner_path: Vec<String>,
    pub owner_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_type: Option<IrRoutineType>,
    pub objects: Vec<AotOwnerObjectArtifact>,
    pub exits: Vec<AotOwnerExitArtifact>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AotPackageArtifact {
    pub format: String,
    pub package_id: String,
    pub package_name: String,
    pub root_module_id: String,
    pub direct_deps: Vec<String>,
    pub direct_dep_ids: Vec<String>,
    pub package_display_names: std::collections::BTreeMap<String, String>,
    pub package_direct_dep_ids:
        std::collections::BTreeMap<String, std::collections::BTreeMap<String, String>>,
    pub module_count: usize,
    pub dependency_edge_count: usize,
    pub dependency_rows: Vec<String>,
    pub exported_surface_rows: Vec<String>,
    pub runtime_requirements: Vec<String>,
    #[serde(default)]
    pub foreword_index: Vec<IrForewordMetadata>,
    #[serde(default)]
    pub foreword_registrations: Vec<IrForewordRegistrationRow>,
    pub entrypoints: Vec<AotEntrypointArtifact>,
    pub routines: Vec<AotRoutineArtifact>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub native_callbacks: Vec<AotNativeCallbackArtifact>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub api_decls: Vec<AotApiDeclArtifact>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shackle_decls: Vec<AotShackleDeclArtifact>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub binding_layouts: Vec<ArcanaCabiBindingLayout>,
    pub owners: Vec<AotOwnerArtifact>,
    pub modules: Vec<AotPackageModuleArtifact>,
}
