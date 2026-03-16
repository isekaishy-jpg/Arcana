use arcana_ir::{ExecPageRollup, ExecStmt};
use serde::{Deserialize, Serialize};

pub const AOT_INTERNAL_FORMAT: &str = "arcana-aot-v4";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AotArtifact {
    pub format: String,
    pub symbol_count: usize,
    pub item_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AotPackageModuleArtifact {
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
    pub module_id: String,
    pub symbol_name: String,
    pub symbol_kind: String,
    pub is_async: bool,
    pub exported: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AotRoutineArtifact {
    pub module_id: String,
    pub routine_key: String,
    pub symbol_name: String,
    pub symbol_kind: String,
    pub exported: bool,
    pub is_async: bool,
    pub type_param_rows: Vec<String>,
    pub behavior_attr_rows: Vec<String>,
    pub param_rows: Vec<String>,
    pub signature_row: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intrinsic_impl: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub impl_target_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub impl_trait_path: Option<Vec<String>>,
    pub foreword_rows: Vec<String>,
    pub rollups: Vec<ExecPageRollup>,
    pub statements: Vec<ExecStmt>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AotPackageArtifact {
    pub format: String,
    pub package_name: String,
    pub root_module_id: String,
    pub direct_deps: Vec<String>,
    pub module_count: usize,
    pub dependency_edge_count: usize,
    pub dependency_rows: Vec<String>,
    pub exported_surface_rows: Vec<String>,
    pub runtime_requirements: Vec<String>,
    pub entrypoints: Vec<AotEntrypointArtifact>,
    pub routines: Vec<AotRoutineArtifact>,
    pub modules: Vec<AotPackageModuleArtifact>,
}
