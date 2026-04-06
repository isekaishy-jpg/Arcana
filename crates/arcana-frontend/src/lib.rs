#![allow(clippy::too_many_arguments)]

use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::rc::Rc;

mod api_fingerprint;
mod semantic_types;
mod surface;
mod trait_contracts;
mod type_resolve;
mod type_validate;
mod where_clause;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use arcana_hir::{
    HirAssignTarget, HirBinaryOp, HirChainStep, HirExpr, HirHeaderAttachment, HirImplDecl,
    HirLocalTypeLookup, HirMatchPattern, HirModule, HirModuleSummary, HirPath, HirPhraseArg,
    HirResolvedModule, HirResolvedTarget, HirResolvedWorkspace, HirStatement, HirStatementKind,
    HirSymbol, HirSymbolBody, HirSymbolKind, HirType, HirTypeKind, HirUnaryOp, HirWorkspacePackage,
    HirWorkspaceSummary, canonicalize_hir_type_in_module, collect_hir_type_refs,
    current_workspace_package_for_module, infer_receiver_expr_type,
    lookup_method_candidates_for_hir_type, lower_module_text,
    match_name_resolves_to_zero_payload_variant, render_symbol_fingerprint,
    render_symbol_signature, resolve_workspace, visible_package_root_for_module,
};
use arcana_ir::{is_runtime_main_entry_symbol, validate_runtime_main_entry_symbol};
use arcana_language_law::{
    HeadedModifierKeyword, MemoryDetailValueKind, MemoryFamily, memory_detail_descriptor,
    memory_family_descriptor, memory_modifier_allowed,
};
use arcana_package::{
    WorkspaceFingerprints, WorkspaceGraph, load_workspace_hir as load_package_workspace_hir,
    load_workspace_hir_from_graph as load_package_workspace_hir_from_graph,
};
use arcana_syntax::{BuiltinOwnershipClass, Span, builtin_ownership_class, is_builtin_type_name};
use semantic_types::{SemanticArena, SemanticLocalBindingId, TypeId};
use surface::{SurfaceSymbolUse, lookup_symbol_path, split_simple_path};
use trait_contracts::validate_impl_trait_where_requirements_structured;
use type_resolve::{canonical_symbol_path, canonical_type_from_path};
use type_validate::{
    validate_boundary_symbol_contract, validate_surface_path_kind, validate_trait_surface,
    validate_type_surface,
};
use where_clause::validate_where_clause_surface;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CheckSummary {
    pub package_count: usize,
    pub module_count: usize,
    pub non_empty_lines: usize,
    pub directive_count: usize,
    pub symbol_count: usize,
    pub warning_count: usize,
}

pub struct CheckedWorkspace {
    summary: CheckSummary,
    pub(crate) workspace: HirWorkspaceSummary,
    pub(crate) resolved_workspace: HirResolvedWorkspace,
    warnings: Vec<CheckWarning>,
    discovered_tests: Vec<DiscoveredTest>,
    foreword_catalog: Vec<ForewordCatalogEntry>,
    foreword_index: Vec<ForewordIndexEntry>,
    foreword_registrations: Vec<ForewordRegistrationRow>,
}

impl CheckedWorkspace {
    pub fn summary(&self) -> &CheckSummary {
        &self.summary
    }

    pub fn into_workspace_parts(self) -> (HirWorkspaceSummary, HirResolvedWorkspace) {
        (self.workspace, self.resolved_workspace)
    }

    pub fn warnings(&self) -> &[CheckWarning] {
        &self.warnings
    }

    pub fn discovered_tests(&self) -> &[DiscoveredTest] {
        &self.discovered_tests
    }

    pub fn foreword_catalog(&self) -> &[ForewordCatalogEntry] {
        &self.foreword_catalog
    }

    pub fn foreword_index(&self) -> &[ForewordIndexEntry] {
        &self.foreword_index
    }

    pub fn foreword_registrations(&self) -> &[ForewordRegistrationRow] {
        &self.foreword_registrations
    }

    pub fn into_summary(self) -> CheckSummary {
        self.summary
    }
}

#[cfg(windows)]
fn displayable_path(path: &Path) -> String {
    let rendered = path.as_os_str().to_string_lossy();
    if let Some(stripped) = rendered.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{stripped}")
    } else if let Some(stripped) = rendered.strip_prefix(r"\\?\") {
        stripped.to_string()
    } else {
        rendered.into_owned()
    }
}

#[cfg(not(windows))]
fn displayable_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CheckWarning {
    pub path: PathBuf,
    pub line: usize,
    pub column: usize,
    pub message: String,
}

impl CheckWarning {
    pub fn render(&self) -> String {
        format!(
            "{}:{}:{}: {}",
            displayable_path(&self.path),
            self.line,
            self.column,
            self.message
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DiscoveredTest {
    pub package_id: String,
    pub module_id: String,
    pub symbol_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ForewordCatalogEntry {
    pub provider_package_id: String,
    pub exposed_name: String,
    pub qualified_name: String,
    pub tier: String,
    pub visibility: String,
    pub action: String,
    pub retention: String,
    pub targets: Vec<String>,
    pub diagnostic_namespace: Option<String>,
    pub handler: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ForewordGeneratedBy {
    pub applied_name: String,
    pub resolved_name: String,
    pub provider_package_id: String,
    pub owner_kind: String,
    pub owner_path: String,
    pub args: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ForewordIndexEntry {
    pub entry_kind: String,
    pub qualified_name: String,
    pub package_id: String,
    pub module_id: String,
    pub target_kind: String,
    pub target_path: String,
    pub retention: String,
    pub args: Vec<String>,
    pub public: bool,
    pub generated_by: Option<ForewordGeneratedBy>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ForewordRegistrationRow {
    pub namespace: String,
    pub key: String,
    pub value: String,
    pub target_kind: String,
    pub target_path: String,
    pub public: bool,
    pub generated_by: ForewordGeneratedBy,
}

#[derive(Clone, Debug)]
enum ResolvedForewordExportKind {
    Builtin,
    User,
}

#[derive(Clone, Debug)]
struct ResolvedForewordExport {
    kind: ResolvedForewordExportKind,
    provider_package_id: String,
    exposed_package_id: String,
    exposed_name: Vec<String>,
    definition: arcana_hir::HirForewordDefinition,
    handler: Option<arcana_hir::HirForewordHandler>,
    public: bool,
}

impl ResolvedForewordExport {
    fn catalog_tier(&self) -> &'static str {
        match self.kind {
            ResolvedForewordExportKind::Builtin => "builtin",
            ResolvedForewordExportKind::User => self.definition.tier.as_str(),
        }
    }

    fn is_builtin(&self) -> bool {
        matches!(self.kind, ResolvedForewordExportKind::Builtin)
    }
}

#[derive(Clone, Debug, Default)]
struct ForewordRegistry {
    exports: BTreeMap<(String, String), ResolvedForewordExport>,
    catalog: Vec<ForewordCatalogEntry>,
    errors: Vec<Diagnostic>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct ExecutedTransformKey {
    package_id: String,
    module_id: String,
    target_kind: String,
    target_path: String,
    line: usize,
    column: usize,
    qualified_name: String,
    args: Vec<String>,
}

#[derive(Clone, Debug)]
struct ExecutedTransform {
    response: ForewordAdapterResponse,
    generated_by: arcana_hir::HirGeneratedByForeword,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", content = "value")]
enum AdapterPayloadValueSnapshot {
    Raw(String),
    Bool(bool),
    Int(i64),
    Str(String),
    Symbol(String),
    Path(Vec<String>),
}

#[derive(Clone, Debug, Serialize)]
struct AdapterPayloadArgSnapshot {
    name: Option<String>,
    rendered: String,
    value: AdapterPayloadValueSnapshot,
}

#[derive(Clone, Debug, Serialize)]
struct AdapterForewordSnapshot {
    applied_name: String,
    resolved_name: String,
    tier: String,
    visibility: String,
    phase: String,
    action: String,
    retention: String,
    targets: Vec<String>,
    diagnostic_namespace: Option<String>,
    payload_schema: Vec<AdapterPayloadFieldSnapshot>,
    repeatable: bool,
    conflicts: Vec<String>,
    args: Vec<AdapterPayloadArgSnapshot>,
    provider_package_id: String,
    exposed_package_id: String,
    handler: Option<String>,
    entry: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct AdapterPayloadFieldSnapshot {
    name: String,
    optional: bool,
    ty: String,
}

#[derive(Clone, Debug, Serialize)]
struct AdapterPackageSnapshot {
    package_id: String,
    package_name: String,
    root_dir: String,
    module_id: String,
}

#[derive(Clone, Debug, Serialize)]
struct AdapterFieldSnapshot {
    name: String,
    ty: String,
    forewords: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct AdapterParamSnapshot {
    mode: Option<String>,
    name: String,
    ty: String,
    forewords: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct AdapterSymbolSnapshot {
    kind: String,
    name: String,
    exported: bool,
    is_async: bool,
    signature: String,
    type_params: Vec<String>,
    params: Vec<AdapterParamSnapshot>,
    return_type: Option<String>,
    forewords: Vec<String>,
    fields: Vec<AdapterFieldSnapshot>,
    methods: Vec<AdapterSymbolSnapshot>,
    variants: Vec<String>,
    assoc_types: Vec<String>,
    body_fingerprint: String,
}

#[derive(Clone, Debug, Serialize)]
struct AdapterDirectiveSnapshot {
    kind: String,
    path: String,
    alias: Option<String>,
    forewords: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct AdapterTargetSnapshot {
    kind: String,
    path: String,
    public: bool,
    owner_kind: String,
    owner_symbol: Option<AdapterSymbolSnapshot>,
    owner_directive: Option<AdapterDirectiveSnapshot>,
    selected_field: Option<AdapterFieldSnapshot>,
    selected_param: Option<AdapterParamSnapshot>,
    selected_method_name: Option<String>,
    container_kind: Option<String>,
    container_name: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct AdapterCatalogEntry {
    exposed_name: String,
    qualified_name: String,
    tier: String,
    action: String,
    retention: String,
    targets: Vec<String>,
    provider_package_id: String,
}

#[derive(Clone, Debug, Serialize)]
struct AdapterArtifactIdentity {
    product_name: String,
    product_path: String,
    runner: Option<String>,
    args: Vec<String>,
    product_digest: Option<String>,
    runner_digest: Option<String>,
}

#[derive(Clone, Debug)]
struct MaterializedForewordAdapterArtifact {
    product_program: String,
    product_arg_path: String,
    runner_program: Option<String>,
    args: Vec<String>,
    product_digest: Option<String>,
    runner_digest: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct ForewordAdapterRequest {
    version: String,
    protocol: String,
    cache_key: String,
    toolchain_version: String,
    dependency_opt_in_enabled: bool,
    package: AdapterPackageSnapshot,
    foreword: AdapterForewordSnapshot,
    target: AdapterTargetSnapshot,
    visible_forewords: Vec<AdapterCatalogEntry>,
    artifact: AdapterArtifactIdentity,
}

#[derive(Clone, Debug, Deserialize)]
struct AdapterResponseArg {
    #[serde(default)]
    name: Option<String>,
    value: String,
}

#[derive(Clone, Debug, Deserialize)]
struct AdapterEmittedMetadataDescriptor {
    #[serde(default)]
    qualified_name: Option<String>,
    #[serde(default)]
    target_kind: Option<String>,
    #[serde(default)]
    target_path: Option<String>,
    #[serde(default)]
    retention: Option<String>,
    #[serde(default)]
    args: Vec<AdapterResponseArg>,
    #[serde(default)]
    public: Option<bool>,
}

#[derive(Clone, Debug, Deserialize)]
struct AdapterRegistrationRowDescriptor {
    namespace: String,
    key: String,
    value: String,
    #[serde(default)]
    target_kind: Option<String>,
    #[serde(default)]
    target_path: Option<String>,
    #[serde(default)]
    public: Option<bool>,
}

#[derive(Clone, Debug, Deserialize)]
struct ForewordAdapterResponse {
    #[serde(default)]
    version: String,
    #[serde(default)]
    diagnostics: Vec<ForewordAdapterDiagnostic>,
    #[serde(default)]
    replace_owner: Option<String>,
    #[serde(default)]
    replace_directive: Option<String>,
    #[serde(default)]
    append_symbols: Vec<String>,
    #[serde(default)]
    append_impls: Vec<String>,
    #[serde(default)]
    emitted_metadata: Vec<AdapterEmittedMetadataDescriptor>,
    #[serde(default)]
    registration_rows: Vec<AdapterRegistrationRowDescriptor>,
}

#[derive(Clone, Debug, Deserialize)]
struct ForewordAdapterDiagnostic {
    severity: String,
    message: String,
    #[serde(default)]
    lint: Option<String>,
}

#[derive(Default)]
struct SemanticValidation {
    errors: Vec<Diagnostic>,
    warnings: Vec<CheckWarning>,
    discovered_tests: Vec<DiscoveredTest>,
    foreword_catalog: Vec<ForewordCatalogEntry>,
    foreword_index: Vec<ForewordIndexEntry>,
    foreword_registrations: Vec<ForewordRegistrationRow>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Diagnostic {
    path: PathBuf,
    line: usize,
    column: usize,
    message: String,
}

impl Diagnostic {
    fn render(&self) -> String {
        format!(
            "{}:{}:{}: {}",
            displayable_path(&self.path),
            self.line,
            self.column,
            self.message
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExprTypeClass {
    Bool,
    Int,
    Str,
    Pair,
    Collection,
}

impl ExprTypeClass {
    const fn label(self) -> &'static str {
        match self {
            Self::Bool => "Bool",
            Self::Int => "Int",
            Self::Str => "Str",
            Self::Pair => "pair",
            Self::Collection => "collection",
        }
    }
}

#[derive(Clone, Debug, Default)]
struct TypeScope {
    type_params: BTreeSet<String>,
    lifetimes: BTreeSet<String>,
    assoc_types: BTreeSet<String>,
    type_param_ids: BTreeMap<String, SemanticLocalBindingId>,
    lifetime_ids: BTreeMap<String, SemanticLocalBindingId>,
    assoc_type_ids: BTreeMap<String, SemanticLocalBindingId>,
    next_binding_id: u32,
    allow_self: bool,
}

impl TypeScope {
    fn with_params(&self, params: &[String]) -> Self {
        let mut next = self.clone();
        for param in params {
            if param.starts_with('\'') {
                next.lifetimes.insert(param.clone());
                next.bind_lifetime(param);
            } else {
                next.type_params.insert(param.clone());
                next.bind_type_param(param);
            }
        }
        next
    }

    fn with_assoc_types<I>(&self, assoc_types: I) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        let mut next = self.clone();
        for assoc_type in assoc_types {
            next.assoc_types.insert(assoc_type.clone());
            next.bind_assoc_type(&assoc_type);
        }
        next
    }

    fn with_self(&self) -> Self {
        let mut next = self.clone();
        next.allow_self = true;
        next
    }

    fn allows_type_name(&self, name: &str) -> bool {
        self.type_params.contains(name)
            || self.assoc_types.contains(name)
            || (self.allow_self && name == "Self")
    }

    fn lifetime_declared(&self, lifetime: &str) -> bool {
        lifetime == "'static" || self.lifetimes.contains(lifetime)
    }

    fn type_param_id(&self, name: &str) -> Option<SemanticLocalBindingId> {
        self.type_param_ids.get(name).copied()
    }

    fn lifetime_id(&self, name: &str) -> Option<SemanticLocalBindingId> {
        self.lifetime_ids.get(name).copied()
    }

    fn assoc_type_id(&self, name: &str) -> Option<SemanticLocalBindingId> {
        self.assoc_type_ids.get(name).copied()
    }

    fn bind_type_param(&mut self, name: &str) -> SemanticLocalBindingId {
        if let Some(existing) = self.type_param_ids.get(name) {
            return *existing;
        }
        let id = SemanticLocalBindingId(self.next_binding_id);
        self.next_binding_id += 1;
        self.type_param_ids.insert(name.to_string(), id);
        id
    }

    fn bind_lifetime(&mut self, name: &str) -> SemanticLocalBindingId {
        if let Some(existing) = self.lifetime_ids.get(name) {
            return *existing;
        }
        let id = SemanticLocalBindingId(self.next_binding_id);
        self.next_binding_id += 1;
        self.lifetime_ids.insert(name.to_string(), id);
        id
    }

    fn bind_assoc_type(&mut self, name: &str) -> SemanticLocalBindingId {
        if let Some(existing) = self.assoc_type_ids.get(name) {
            return *existing;
        }
        let id = SemanticLocalBindingId(self.next_binding_id);
        self.next_binding_id += 1;
        self.assoc_type_ids.insert(name.to_string(), id);
        id
    }
}

#[derive(Clone, Debug)]
struct ValueScope {
    locals: BTreeSet<String>,
    mutable_locals: BTreeSet<String>,
    params: BTreeSet<String>,
    ownership: BTreeMap<String, OwnershipClass>,
    types: BTreeMap<String, HirType>,
    binding_ids: BTreeMap<String, u64>,
    next_binding_id: Rc<Cell<u64>>,
    memory_specs: BTreeMap<String, VisibleMemorySpecBinding>,
    available_owners: BTreeMap<String, AvailableOwnerBinding>,
    active_owners: BTreeMap<String, AvailableOwnerBinding>,
    attached_object_names: BTreeSet<String>,
    owner_member_types: BTreeMap<String, BTreeMap<String, HirType>>,
    loop_depth: usize,
    headed_region_depth: usize,
    enclosing_return_type: Option<HirType>,
}

impl Default for ValueScope {
    fn default() -> Self {
        Self {
            locals: BTreeSet::new(),
            mutable_locals: BTreeSet::new(),
            params: BTreeSet::new(),
            ownership: BTreeMap::new(),
            types: BTreeMap::new(),
            binding_ids: BTreeMap::new(),
            next_binding_id: Rc::new(Cell::new(1)),
            memory_specs: BTreeMap::new(),
            available_owners: BTreeMap::new(),
            active_owners: BTreeMap::new(),
            attached_object_names: BTreeSet::new(),
            owner_member_types: BTreeMap::new(),
            loop_depth: 0,
            headed_region_depth: 0,
            enclosing_return_type: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AvailableOwnerObjectBinding {
    local_name: String,
    ty: HirType,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AvailableOwnerBinding {
    local_name: String,
    owner_path: Vec<String>,
    objects: Vec<AvailableOwnerObjectBinding>,
    exit_names: BTreeSet<String>,
    activation_context_type: Option<HirType>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct VisibleMemorySpecBinding {
    family: MemoryFamily,
    span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum LifecycleHookSlot {
    Init,
    InitWithContext,
    Resume,
    ResumeWithContext,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct ObjectLifecycleSurface {
    init_context_type: Option<HirType>,
    resume_context_type: Option<HirType>,
}

impl ValueScope {
    fn with_symbol_params(&self, params: &[arcana_hir::HirParam]) -> Self {
        let mut next = self.clone();
        for param in params {
            next.bind_local(
                &param.name,
                matches!(param.mode, Some(arcana_hir::HirParamMode::Edit)),
            );
            next.params.insert(param.name.clone());
        }
        next
    }

    fn insert(&mut self, name: &str, mutable: bool) {
        self.bind_local(name, mutable);
    }

    fn bind_local(&mut self, name: &str, mutable: bool) -> u64 {
        self.locals.insert(name.to_string());
        if mutable {
            self.mutable_locals.insert(name.to_string());
        }
        let binding_id = self.next_binding_id.get();
        self.next_binding_id.set(binding_id + 1);
        self.binding_ids.insert(name.to_string(), binding_id);
        binding_id
    }

    fn insert_typed(
        &mut self,
        name: &str,
        mutable: bool,
        ownership: OwnershipClass,
        ty: Option<HirType>,
    ) {
        self.insert(name, mutable);
        self.ownership.insert(name.to_string(), ownership);
        if let Some(ty) = ty {
            self.types.insert(name.to_string(), ty);
        } else {
            self.types.remove(name);
        }
    }

    fn contains(&self, name: &str) -> bool {
        self.locals.contains(name)
    }

    fn is_mutable(&self, name: &str) -> bool {
        self.mutable_locals.contains(name)
    }

    fn is_param(&self, name: &str) -> bool {
        self.params.contains(name)
    }

    fn ownership_of(&self, name: &str) -> OwnershipClass {
        self.ownership.get(name).copied().unwrap_or_default()
    }

    fn type_of(&self, name: &str) -> Option<&HirType> {
        self.types.get(name)
    }

    fn binding_id_of(&self, name: &str) -> Option<u64> {
        self.binding_ids.get(name).copied()
    }

    fn attach_owner(&mut self, owner: AvailableOwnerBinding) {
        self.available_owners
            .insert(owner.local_name.clone(), owner);
    }

    fn insert_memory_spec(&mut self, name: &str, family: MemoryFamily, span: Span) {
        self.memory_specs
            .insert(name.to_string(), VisibleMemorySpecBinding { family, span });
    }

    fn memory_spec(&self, name: &str) -> Option<&VisibleMemorySpecBinding> {
        self.memory_specs.get(name)
    }

    fn active_owner_for_exit(
        &self,
        exit_name: &str,
    ) -> Result<Option<&AvailableOwnerBinding>, String> {
        let matches = self
            .active_owners
            .values()
            .filter(|owner| owner.exit_names.contains(exit_name))
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [] => Ok(None),
            [owner] => Ok(Some(*owner)),
            _ => Err(format!(
                "named recycle exit `{exit_name}` is ambiguous across active owners"
            )),
        }
    }

    fn attach_object_name(&mut self, name: impl Into<String>) {
        self.attached_object_names.insert(name.into());
    }

    fn owner_member_type(&self, owner_name: &str, member: &str) -> Option<&HirType> {
        self.owner_member_types
            .get(owner_name)
            .and_then(|members| members.get(member))
    }

    fn activate_owner(
        &mut self,
        owner: &AvailableOwnerBinding,
        explicit_binding: Option<&str>,
        explicit_binding_mutable: bool,
    ) -> Vec<String> {
        let mut inserted = Vec::new();
        let mut owner_members = BTreeMap::new();
        for object in &owner.objects {
            owner_members.insert(object.local_name.clone(), object.ty.clone());
            if self.attached_object_names.contains(&object.local_name) {
                self.insert_typed(
                    &object.local_name,
                    true,
                    OwnershipClass::Move,
                    Some(object.ty.clone()),
                );
                inserted.push(object.local_name.clone());
            }
        }
        self.insert_typed(&owner.local_name, false, OwnershipClass::Copy, None);
        self.active_owners
            .insert(owner.owner_path.join("."), owner.clone());
        self.owner_member_types
            .insert(owner.local_name.clone(), owner_members.clone());
        inserted.push(owner.local_name.clone());
        if let Some(binding) = explicit_binding {
            self.insert_typed(
                binding,
                explicit_binding_mutable,
                OwnershipClass::Copy,
                None,
            );
            self.owner_member_types
                .insert(binding.to_string(), owner_members);
            inserted.push(binding.to_string());
        }
        inserted
    }
}

impl HirLocalTypeLookup for ValueScope {
    fn contains_local(&self, name: &str) -> bool {
        ValueScope::contains(self, name)
    }

    fn type_of(&self, name: &str) -> Option<&HirType> {
        ValueScope::type_of(self, name)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum OwnershipClass {
    Copy,
    Move,
    #[default]
    Unknown,
}

impl OwnershipClass {
    const fn is_move_only(self) -> bool {
        matches!(self, Self::Move)
    }
}

fn infer_expr_type(expr: &HirExpr) -> Option<ExprTypeClass> {
    match expr {
        HirExpr::BoolLiteral { .. } => Some(ExprTypeClass::Bool),
        HirExpr::IntLiteral { .. } => Some(ExprTypeClass::Int),
        HirExpr::StrLiteral { .. } => Some(ExprTypeClass::Str),
        HirExpr::Pair { .. } => Some(ExprTypeClass::Pair),
        HirExpr::CollectionLiteral { .. } => Some(ExprTypeClass::Collection),
        HirExpr::Unary { op, .. } => match op {
            HirUnaryOp::Not => Some(ExprTypeClass::Bool),
            HirUnaryOp::Neg | HirUnaryOp::BitNot => Some(ExprTypeClass::Int),
            HirUnaryOp::BorrowRead
            | HirUnaryOp::BorrowMut
            | HirUnaryOp::Deref
            | HirUnaryOp::Weave
            | HirUnaryOp::Split => None,
        },
        HirExpr::Binary { op, .. } => match op {
            HirBinaryOp::And
            | HirBinaryOp::Or
            | HirBinaryOp::EqEq
            | HirBinaryOp::NotEq
            | HirBinaryOp::Lt
            | HirBinaryOp::LtEq
            | HirBinaryOp::Gt
            | HirBinaryOp::GtEq => Some(ExprTypeClass::Bool),
            HirBinaryOp::Sub
            | HirBinaryOp::Mul
            | HirBinaryOp::Div
            | HirBinaryOp::Mod
            | HirBinaryOp::BitOr
            | HirBinaryOp::BitXor
            | HirBinaryOp::BitAnd
            | HirBinaryOp::Shl
            | HirBinaryOp::Shr => Some(ExprTypeClass::Int),
            HirBinaryOp::Add => None,
        },
        _ => None,
    }
}

fn builtin_scalar_expr_type(expr_type: ExprTypeClass) -> Option<HirType> {
    let name = match expr_type {
        ExprTypeClass::Bool => "Bool",
        ExprTypeClass::Int => "Int",
        ExprTypeClass::Str => "Str",
        ExprTypeClass::Pair | ExprTypeClass::Collection => return None,
    };
    Some(HirType {
        kind: HirTypeKind::Path(HirPath {
            segments: vec![name.to_string()],
            span: Span::default(),
        }),
        span: Span::default(),
    })
}

fn push_type_contract_diagnostic(
    module_path: &Path,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
    message: String,
) {
    diagnostics.push(Diagnostic {
        path: module_path.to_path_buf(),
        line: span.line,
        column: span.column,
        message,
    });
}

fn validate_expected_expr_type(
    module_path: &Path,
    expr: &HirExpr,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
    expected: ExprTypeClass,
    context: &str,
) {
    let Some(actual) = infer_expr_type(expr) else {
        return;
    };
    if actual != expected {
        push_type_contract_diagnostic(
            module_path,
            span,
            diagnostics,
            format!(
                "{context} requires {}, found {}",
                expected.label(),
                actual.label()
            ),
        );
    }
}

fn is_tuple_projection_member(member: &str) -> bool {
    matches!(member, "0" | "1")
}

fn binary_op_token(op: HirBinaryOp) -> &'static str {
    match op {
        HirBinaryOp::Or => "or",
        HirBinaryOp::And => "and",
        HirBinaryOp::EqEq => "==",
        HirBinaryOp::NotEq => "!=",
        HirBinaryOp::Lt => "<",
        HirBinaryOp::LtEq => "<=",
        HirBinaryOp::Gt => ">",
        HirBinaryOp::GtEq => ">=",
        HirBinaryOp::BitOr => "|",
        HirBinaryOp::BitXor => "^",
        HirBinaryOp::BitAnd => "&",
        HirBinaryOp::Shl => "<<",
        HirBinaryOp::Shr => "shr",
        HirBinaryOp::Add => "+",
        HirBinaryOp::Sub => "-",
        HirBinaryOp::Mul => "*",
        HirBinaryOp::Div => "/",
        HirBinaryOp::Mod => "%",
    }
}

fn unary_op_token(op: HirUnaryOp) -> &'static str {
    match op {
        HirUnaryOp::Neg => "-",
        HirUnaryOp::Not => "not",
        HirUnaryOp::BitNot => "~",
        HirUnaryOp::BorrowRead => "&",
        HirUnaryOp::BorrowMut => "*",
        HirUnaryOp::Deref => "*",
        HirUnaryOp::Weave => "weave",
        HirUnaryOp::Split => "split",
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PlaceMutability {
    Immutable,
    Mutable,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BorrowedSliceSurfaceKind {
    Array,
    List,
    ReadView,
    EditView,
    ByteView,
    ByteEditView,
    Str,
    StrView,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct LocalBorrowState {
    shared_count: usize,
    mutable: bool,
}

#[derive(Clone, Debug, Default)]
struct BorrowFlowState {
    locals: BTreeMap<String, LocalBorrowState>,
    moved_locals: BTreeSet<String>,
    active_cleanup_bindings: BTreeSet<u64>,
}

impl BorrowFlowState {
    fn local_state(&self, name: &str) -> LocalBorrowState {
        self.locals.get(name).copied().unwrap_or_default()
    }

    fn has_shared_borrow(&self, name: &str) -> bool {
        self.local_state(name).shared_count > 0
    }

    fn has_mut_borrow(&self, name: &str) -> bool {
        self.local_state(name).mutable
    }

    fn has_any_borrow(&self, name: &str) -> bool {
        let state = self.local_state(name);
        state.mutable || state.shared_count > 0
    }

    fn has_moved(&self, name: &str) -> bool {
        self.moved_locals.contains(name)
    }

    fn note_shared_borrow(&mut self, name: &str) {
        let state = self.locals.entry(name.to_string()).or_default();
        state.shared_count += 1;
    }

    fn note_mut_borrow(&mut self, name: &str) {
        let state = self.locals.entry(name.to_string()).or_default();
        state.mutable = true;
    }

    fn note_moved(&mut self, name: &str) {
        self.moved_locals.insert(name.to_string());
    }

    fn has_active_cleanup_binding(&self, binding_id: u64) -> bool {
        self.active_cleanup_bindings.contains(&binding_id)
    }

    fn activate_cleanup_binding(&mut self, binding_id: u64) {
        self.active_cleanup_bindings.insert(binding_id);
    }

    fn clear_local(&mut self, name: &str) {
        self.locals.remove(name);
        self.moved_locals.remove(name);
    }

    fn merge_moves_from(&mut self, other: &Self) {
        self.moved_locals.extend(other.moved_locals.iter().cloned());
    }
}

fn hir_type_matches_path(ty: &HirType, expected: &[&str]) -> bool {
    match &ty.kind {
        HirTypeKind::Path(path) => {
            path.segments.len() == expected.len()
                && path
                    .segments
                    .iter()
                    .map(String::as_str)
                    .eq(expected.iter().copied())
        }
        HirTypeKind::Apply { base, .. } => {
            base.segments.len() == expected.len()
                && base
                    .segments
                    .iter()
                    .map(String::as_str)
                    .eq(expected.iter().copied())
        }
        HirTypeKind::Ref { inner, .. } => hir_type_matches_path(inner, expected),
        _ => false,
    }
}

fn classify_borrowed_slice_surface(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    scope: &ValueScope,
    expr: &HirExpr,
) -> BorrowedSliceSurfaceKind {
    let Some(ty) = infer_receiver_expr_type(workspace, resolved_module, scope, expr) else {
        return BorrowedSliceSurfaceKind::Unsupported;
    };
    if hir_type_matches_path(&ty, &["Array"])
        || hir_type_matches_path(&ty, &["std", "collections", "array", "Array"])
    {
        return BorrowedSliceSurfaceKind::Array;
    }
    if hir_type_matches_path(&ty, &["List"])
        || hir_type_matches_path(&ty, &["std", "collections", "list", "List"])
    {
        return BorrowedSliceSurfaceKind::List;
    }
    if hir_type_matches_path(&ty, &["ReadView"])
        || hir_type_matches_path(&ty, &["std", "memory", "ReadView"])
    {
        return BorrowedSliceSurfaceKind::ReadView;
    }
    if hir_type_matches_path(&ty, &["EditView"])
        || hir_type_matches_path(&ty, &["std", "memory", "EditView"])
    {
        return BorrowedSliceSurfaceKind::EditView;
    }
    if hir_type_matches_path(&ty, &["ByteView"])
        || hir_type_matches_path(&ty, &["std", "memory", "ByteView"])
    {
        return BorrowedSliceSurfaceKind::ByteView;
    }
    if hir_type_matches_path(&ty, &["ByteEditView"])
        || hir_type_matches_path(&ty, &["std", "memory", "ByteEditView"])
    {
        return BorrowedSliceSurfaceKind::ByteEditView;
    }
    if hir_type_matches_path(&ty, &["Str"]) {
        return BorrowedSliceSurfaceKind::Str;
    }
    if hir_type_matches_path(&ty, &["StrView"])
        || hir_type_matches_path(&ty, &["std", "memory", "StrView"])
    {
        return BorrowedSliceSurfaceKind::StrView;
    }
    BorrowedSliceSurfaceKind::Unsupported
}

fn expr_place_mutability(expr: &HirExpr, scope: &ValueScope) -> Option<PlaceMutability> {
    match expr {
        HirExpr::Path { segments } if segments.len() == 1 && scope.contains(&segments[0]) => {
            Some(if scope.is_mutable(&segments[0]) {
                PlaceMutability::Mutable
            } else {
                PlaceMutability::Immutable
            })
        }
        HirExpr::MemberAccess { expr, .. }
        | HirExpr::Index { expr, .. }
        | HirExpr::Slice { expr, .. } => expr_place_mutability(expr, scope),
        HirExpr::Unary {
            op: HirUnaryOp::Deref,
            ..
        } => Some(PlaceMutability::Unknown),
        _ => None,
    }
}

fn expr_place_root_local<'a>(expr: &'a HirExpr, scope: &ValueScope) -> Option<&'a str> {
    match expr {
        HirExpr::Path { segments } if segments.len() == 1 && scope.contains(&segments[0]) => {
            Some(segments[0].as_str())
        }
        HirExpr::MemberAccess { expr, .. }
        | HirExpr::Index { expr, .. }
        | HirExpr::Slice { expr, .. } => expr_place_root_local(expr, scope),
        _ => None,
    }
}

fn assign_target_root_local<'a>(
    target: &'a HirAssignTarget,
    scope: &ValueScope,
) -> Option<&'a str> {
    match target {
        HirAssignTarget::Name { text } if scope.contains(text) => Some(text.as_str()),
        HirAssignTarget::MemberAccess { target, .. } | HirAssignTarget::Index { target, .. } => {
            assign_target_root_local(target, scope)
        }
        _ => None,
    }
}

fn ownership_of_builtin_type(name: &str) -> OwnershipClass {
    match builtin_ownership_class(name) {
        Some(BuiltinOwnershipClass::Copy) => OwnershipClass::Copy,
        Some(BuiltinOwnershipClass::Move) => OwnershipClass::Move,
        None => OwnershipClass::Unknown,
    }
}

fn ownership_of_opaque_symbol(symbol: &HirSymbol) -> OwnershipClass {
    match symbol.opaque_policy.map(|policy| policy.ownership) {
        Some(arcana_hir::HirOpaqueOwnershipPolicy::Copy) => OwnershipClass::Copy,
        Some(arcana_hir::HirOpaqueOwnershipPolicy::Move) => OwnershipClass::Move,
        None => OwnershipClass::Unknown,
    }
}

fn opaque_symbol_is_boundary_unsafe(symbol: &HirSymbol) -> bool {
    matches!(
        symbol.opaque_policy.map(|policy| policy.boundary),
        Some(arcana_hir::HirOpaqueBoundaryPolicy::Unsafe)
    )
}

#[allow(clippy::enum_variant_names)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OpaqueLangFamily {
    FileStreamHandle,
    WindowHandle,
    ImageHandle,
    AppFrameHandle,
    AppSessionHandle,
    WakeHandle,
    AudioDeviceHandle,
    AudioBufferHandle,
    AudioPlaybackHandle,
    ChannelHandle,
    MutexHandle,
    AtomicIntHandle,
    AtomicBoolHandle,
    ArenaHandle,
    ArenaIdHandle,
    FrameArenaHandle,
    FrameIdHandle,
    PoolArenaHandle,
    PoolIdHandle,
    TempArenaHandle,
    TempIdHandle,
    SessionArenaHandle,
    SessionIdHandle,
    RingBufferHandle,
    RingIdHandle,
    SlabHandle,
    SlabIdHandle,
    ReadViewHandle,
    EditViewHandle,
    ByteViewHandle,
    ByteEditViewHandle,
    StrViewHandle,
    TaskHandle,
    ThreadHandle,
}

impl OpaqueLangFamily {
    const fn name(self) -> &'static str {
        match self {
            Self::FileStreamHandle => "file_stream_handle",
            Self::WindowHandle => "window_handle",
            Self::ImageHandle => "image_handle",
            Self::AppFrameHandle => "app_frame_handle",
            Self::AppSessionHandle => "app_session_handle",
            Self::WakeHandle => "wake_handle",
            Self::AudioDeviceHandle => "audio_device_handle",
            Self::AudioBufferHandle => "audio_buffer_handle",
            Self::AudioPlaybackHandle => "audio_playback_handle",
            Self::ChannelHandle => "channel_handle",
            Self::MutexHandle => "mutex_handle",
            Self::AtomicIntHandle => "atomic_int_handle",
            Self::AtomicBoolHandle => "atomic_bool_handle",
            Self::ArenaHandle => "arena_handle",
            Self::ArenaIdHandle => "arena_id_handle",
            Self::FrameArenaHandle => "frame_arena_handle",
            Self::FrameIdHandle => "frame_id_handle",
            Self::PoolArenaHandle => "pool_arena_handle",
            Self::PoolIdHandle => "pool_id_handle",
            Self::TempArenaHandle => "temp_arena_handle",
            Self::TempIdHandle => "temp_id_handle",
            Self::SessionArenaHandle => "session_arena_handle",
            Self::SessionIdHandle => "session_id_handle",
            Self::RingBufferHandle => "ring_buffer_handle",
            Self::RingIdHandle => "ring_id_handle",
            Self::SlabHandle => "slab_handle",
            Self::SlabIdHandle => "slab_id_handle",
            Self::ReadViewHandle => "read_view_handle",
            Self::EditViewHandle => "edit_view_handle",
            Self::ByteViewHandle => "byte_view_handle",
            Self::ByteEditViewHandle => "byte_edit_view_handle",
            Self::StrViewHandle => "str_view_handle",
            Self::TaskHandle => "task_handle",
            Self::ThreadHandle => "thread_handle",
        }
    }

    const fn expected_ownership(self) -> OwnershipClass {
        match self {
            Self::WakeHandle
            | Self::AtomicIntHandle
            | Self::AtomicBoolHandle
            | Self::ArenaIdHandle
            | Self::FrameIdHandle
            | Self::PoolIdHandle
            | Self::TempIdHandle
            | Self::SessionIdHandle
            | Self::RingIdHandle
            | Self::SlabIdHandle => OwnershipClass::Copy,
            Self::FileStreamHandle
            | Self::WindowHandle
            | Self::ImageHandle
            | Self::AppFrameHandle
            | Self::AppSessionHandle
            | Self::AudioDeviceHandle
            | Self::AudioBufferHandle
            | Self::AudioPlaybackHandle
            | Self::ChannelHandle
            | Self::MutexHandle
            | Self::ArenaHandle
            | Self::FrameArenaHandle
            | Self::PoolArenaHandle
            | Self::TempArenaHandle
            | Self::SessionArenaHandle
            | Self::RingBufferHandle
            | Self::SlabHandle
            | Self::ReadViewHandle
            | Self::EditViewHandle
            | Self::ByteViewHandle
            | Self::ByteEditViewHandle
            | Self::StrViewHandle
            | Self::TaskHandle
            | Self::ThreadHandle => OwnershipClass::Move,
        }
    }
}

fn opaque_lang_family(name: &str) -> Option<OpaqueLangFamily> {
    match name {
        "file_stream_handle" => Some(OpaqueLangFamily::FileStreamHandle),
        "window_handle" => Some(OpaqueLangFamily::WindowHandle),
        "image_handle" => Some(OpaqueLangFamily::ImageHandle),
        "app_frame_handle" => Some(OpaqueLangFamily::AppFrameHandle),
        "app_session_handle" => Some(OpaqueLangFamily::AppSessionHandle),
        "wake_handle" => Some(OpaqueLangFamily::WakeHandle),
        "audio_device_handle" => Some(OpaqueLangFamily::AudioDeviceHandle),
        "audio_buffer_handle" => Some(OpaqueLangFamily::AudioBufferHandle),
        "audio_playback_handle" => Some(OpaqueLangFamily::AudioPlaybackHandle),
        "channel_handle" => Some(OpaqueLangFamily::ChannelHandle),
        "mutex_handle" => Some(OpaqueLangFamily::MutexHandle),
        "atomic_int_handle" => Some(OpaqueLangFamily::AtomicIntHandle),
        "atomic_bool_handle" => Some(OpaqueLangFamily::AtomicBoolHandle),
        "arena_handle" => Some(OpaqueLangFamily::ArenaHandle),
        "arena_id_handle" => Some(OpaqueLangFamily::ArenaIdHandle),
        "frame_arena_handle" => Some(OpaqueLangFamily::FrameArenaHandle),
        "frame_id_handle" => Some(OpaqueLangFamily::FrameIdHandle),
        "pool_arena_handle" => Some(OpaqueLangFamily::PoolArenaHandle),
        "pool_id_handle" => Some(OpaqueLangFamily::PoolIdHandle),
        "temp_arena_handle" => Some(OpaqueLangFamily::TempArenaHandle),
        "temp_id_handle" => Some(OpaqueLangFamily::TempIdHandle),
        "session_arena_handle" => Some(OpaqueLangFamily::SessionArenaHandle),
        "session_id_handle" => Some(OpaqueLangFamily::SessionIdHandle),
        "ring_buffer_handle" => Some(OpaqueLangFamily::RingBufferHandle),
        "ring_id_handle" => Some(OpaqueLangFamily::RingIdHandle),
        "slab_handle" => Some(OpaqueLangFamily::SlabHandle),
        "slab_id_handle" => Some(OpaqueLangFamily::SlabIdHandle),
        "read_view_handle" => Some(OpaqueLangFamily::ReadViewHandle),
        "edit_view_handle" => Some(OpaqueLangFamily::EditViewHandle),
        "byte_view_handle" => Some(OpaqueLangFamily::ByteViewHandle),
        "byte_edit_view_handle" => Some(OpaqueLangFamily::ByteEditViewHandle),
        "str_view_handle" => Some(OpaqueLangFamily::StrViewHandle),
        "task_handle" => Some(OpaqueLangFamily::TaskHandle),
        "thread_handle" => Some(OpaqueLangFamily::ThreadHandle),
        _ => None,
    }
}

fn validate_package_lang_item_semantics(
    package: &HirWorkspacePackage,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut seen = BTreeMap::<String, (PathBuf, usize, usize)>::new();
    for module in &package.summary.modules {
        let module_path = package
            .module_path(&module.module_id)
            .cloned()
            .unwrap_or_else(|| package.root_dir.join("src").join("unknown.arc"));
        for lang_item in &module.lang_items {
            let Some(family) = opaque_lang_family(&lang_item.name) else {
                continue;
            };
            if let Some((prev_path, prev_line, prev_column)) = seen.insert(
                lang_item.name.clone(),
                (
                    module_path.clone(),
                    lang_item.span.line,
                    lang_item.span.column,
                ),
            ) {
                diagnostics.push(Diagnostic {
                    path: module_path.clone(),
                    line: lang_item.span.line,
                    column: lang_item.span.column,
                    message: format!(
                        "opaque family lang item `{}` is declared more than once in package `{}`; first seen at {}:{}:{}",
                        family.name(),
                        package.summary.package_name,
                        prev_path.display(),
                        prev_line,
                        prev_column
                    ),
                });
            }
        }
    }
}

fn infer_type_ownership(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    type_scope: &TypeScope,
    ty: &HirType,
) -> OwnershipClass {
    match &ty.kind {
        arcana_hir::HirTypeKind::Ref { .. } => OwnershipClass::Copy,
        arcana_hir::HirTypeKind::Path(path) => {
            if path.segments.len() == 1 && type_scope.allows_type_name(&path.segments[0]) {
                return OwnershipClass::Unknown;
            }
            if path.segments.len() == 1 {
                let builtin = ownership_of_builtin_type(&path.segments[0]);
                if builtin != OwnershipClass::Unknown {
                    return builtin;
                }
            }
            let Some(symbol_ref) = lookup_symbol_path(workspace, resolved_module, &path.segments)
            else {
                return OwnershipClass::Unknown;
            };
            match symbol_ref.symbol.kind {
                HirSymbolKind::OpaqueType => ownership_of_opaque_symbol(symbol_ref.symbol),
                HirSymbolKind::Record | HirSymbolKind::Object | HirSymbolKind::Enum => {
                    OwnershipClass::Move
                }
                _ => OwnershipClass::Unknown,
            }
        }
        arcana_hir::HirTypeKind::Apply { base, .. } => infer_type_ownership(
            workspace,
            resolved_module,
            type_scope,
            &HirType {
                kind: arcana_hir::HirTypeKind::Path(base.clone()),
                span: ty.span,
            },
        ),
        _ => OwnershipClass::Unknown,
    }
}

fn infer_expr_value_type(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    _type_scope: &TypeScope,
    scope: &ValueScope,
    expr: &HirExpr,
) -> Option<HirType> {
    if let HirExpr::MemberAccess { expr, member } = expr
        && let HirExpr::Path { segments } = expr.as_ref()
        && segments.len() == 1
        && scope.contains(&segments[0])
        && let Some(ty) = scope.owner_member_type(&segments[0], member)
    {
        return Some(ty.clone());
    }
    if let HirExpr::ConstructRegion(region) = expr
        && let Some(path) = flatten_callable_expr_path(&region.target)
    {
        return resolve_construct_result_type(workspace, resolved_module, &path);
    }
    if let HirExpr::RecordRegion(region) = expr
        && let Some(path) = flatten_callable_expr_path(&region.target)
    {
        return resolve_record_result_type(workspace, resolved_module, &path);
    }
    infer_receiver_expr_type(workspace, resolved_module, scope, expr)
        .or_else(|| infer_expr_type(expr).and_then(builtin_scalar_expr_type))
}

fn assign_target_to_expr(target: &HirAssignTarget) -> HirExpr {
    match target {
        HirAssignTarget::Name { text } => HirExpr::Path {
            segments: vec![text.clone()],
        },
        HirAssignTarget::MemberAccess { target, member } => HirExpr::MemberAccess {
            expr: Box::new(assign_target_to_expr(target)),
            member: member.clone(),
        },
        HirAssignTarget::Index { target, index } => HirExpr::Index {
            expr: Box::new(assign_target_to_expr(target)),
            index: Box::new(index.clone()),
        },
    }
}

fn infer_assign_target_value_type(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    type_scope: &TypeScope,
    scope: &ValueScope,
    target: &HirAssignTarget,
) -> Option<HirType> {
    let expr = assign_target_to_expr(target);
    infer_expr_value_type(workspace, resolved_module, type_scope, scope, &expr)
}

fn infer_expr_ownership(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    type_scope: &TypeScope,
    scope: &ValueScope,
    expr: &HirExpr,
) -> OwnershipClass {
    match expr {
        HirExpr::BoolLiteral { .. } | HirExpr::IntLiteral { .. } => OwnershipClass::Copy,
        HirExpr::StrLiteral { .. } | HirExpr::CollectionLiteral { .. } => OwnershipClass::Move,
        HirExpr::Pair { left, right } => {
            let left_kind =
                infer_expr_ownership(workspace, resolved_module, type_scope, scope, left);
            let right_kind =
                infer_expr_ownership(workspace, resolved_module, type_scope, scope, right);
            if left_kind == OwnershipClass::Copy && right_kind == OwnershipClass::Copy {
                OwnershipClass::Copy
            } else if left_kind.is_move_only() || right_kind.is_move_only() {
                OwnershipClass::Move
            } else {
                OwnershipClass::Unknown
            }
        }
        HirExpr::Path { segments } if segments.len() == 1 && scope.contains(&segments[0]) => {
            scope.ownership_of(&segments[0])
        }
        HirExpr::Unary {
            op: HirUnaryOp::BorrowRead | HirUnaryOp::BorrowMut,
            ..
        } => OwnershipClass::Copy,
        _ => infer_expr_value_type(workspace, resolved_module, type_scope, scope, expr)
            .map(|ty| infer_type_ownership(workspace, resolved_module, type_scope, &ty))
            .unwrap_or_default(),
    }
}

fn flatten_callable_expr_path(expr: &HirExpr) -> Option<Vec<String>> {
    match expr {
        HirExpr::GenericApply { expr, .. } => flatten_callable_expr_path(expr),
        _ => flatten_member_expr_path(expr),
    }
}

fn format_bare_method_ambiguity(ty: &HirType, method_name: &str, symbols: &[&HirSymbol]) -> String {
    let rendered = symbols
        .iter()
        .map(|symbol| render_symbol_signature(symbol))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "bare-method qualifier `{method_name}` on `{}` is ambiguous; candidates: {rendered}",
        ty.render()
    )
}

fn lookup_method_symbol_for_type<'a>(
    workspace: &'a HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    ty: &HirType,
    method_name: &str,
) -> Result<Option<&'a HirSymbol>, String> {
    let candidates =
        lookup_method_candidates_for_hir_type(workspace, resolved_module, ty, method_name)
            .into_iter()
            .map(|candidate| candidate.symbol)
            .collect::<Vec<_>>();
    match candidates.as_slice() {
        [] => Ok(None),
        [symbol] => Ok(Some(*symbol)),
        _ => Err(format_bare_method_ambiguity(ty, method_name, &candidates)),
    }
}

fn resolve_qualified_phrase_target_symbol<'a>(
    workspace: &'a HirWorkspaceSummary,
    resolved_module: &'a HirResolvedModule,
    _type_scope: &TypeScope,
    scope: &ValueScope,
    subject: &HirExpr,
    qualifier_kind: arcana_hir::HirQualifiedPhraseQualifierKind,
    qualifier: &str,
) -> Option<&'a HirSymbol> {
    match qualifier_kind {
        arcana_hir::HirQualifiedPhraseQualifierKind::Call
        | arcana_hir::HirQualifiedPhraseQualifierKind::Weave
        | arcana_hir::HirQualifiedPhraseQualifierKind::Split => {
            let path = flatten_callable_expr_path(subject)?;
            lookup_symbol_path(workspace, resolved_module, &path).map(|resolved| resolved.symbol)
        }
        arcana_hir::HirQualifiedPhraseQualifierKind::NamedPath => {
            let path = split_simple_path(qualifier)?;
            lookup_symbol_path(workspace, resolved_module, &path).map(|resolved| resolved.symbol)
        }
        arcana_hir::HirQualifiedPhraseQualifierKind::BareMethod => {
            let subject_ty = infer_receiver_expr_type(workspace, resolved_module, scope, subject)?;
            lookup_method_symbol_for_type(workspace, resolved_module, &subject_ty, qualifier)
                .ok()
                .flatten()
        }
        _ => None,
    }
}

struct ResolvedOwnerActivation<'a> {
    owner: AvailableOwnerBinding,
    context: Option<&'a HirExpr>,
    invalid: Option<String>,
}

fn resolve_owner_activation_expr<'a>(
    workspace: &HirWorkspaceSummary,
    resolved_workspace: &HirResolvedWorkspace,
    resolved_module: &HirResolvedModule,
    expr: &'a HirExpr,
) -> Option<ResolvedOwnerActivation<'a>> {
    let HirExpr::QualifiedPhrase {
        subject,
        args,
        qualifier_kind,
        qualifier: _,
        ..
    } = expr
    else {
        return None;
    };
    if *qualifier_kind != arcana_hir::HirQualifiedPhraseQualifierKind::Call {
        return None;
    }
    let path = flatten_callable_expr_path(subject)?;
    let resolved = lookup_symbol_path(workspace, resolved_module, &path)?;
    if resolved.symbol.kind != HirSymbolKind::Owner {
        return None;
    }
    let owner =
        resolve_available_owner_binding(workspace, resolved_workspace, resolved_module, &path)?;
    let invalid = if args.iter().any(|arg| matches!(arg, HirPhraseArg::Named { .. })) {
        Some("owner activation does not support named arguments".to_string())
    } else if args.len() > 1 {
        Some("owner activation accepts at most one context argument".to_string())
    } else if owner.activation_context_type.is_some() && args.is_empty() {
        Some("owner activation requires exactly one context argument".to_string())
    } else if owner.activation_context_type.is_none() && !args.is_empty() {
        Some("owner activation does not use an activation context".to_string())
    } else {
        None
    };
    let context = args.first().and_then(|arg| match arg {
        HirPhraseArg::Positional(expr) => Some(expr),
        HirPhraseArg::Named { .. } => None,
    });
    Some(ResolvedOwnerActivation {
        owner,
        context,
        invalid,
    })
}

fn validate_owner_activation_context(
    workspace: &HirWorkspaceSummary,
    _resolved_workspace: &HirResolvedWorkspace,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    type_scope: &TypeScope,
    scope: &ValueScope,
    owner_activation: &ResolvedOwnerActivation<'_>,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(context) = owner_activation.context else {
        return;
    };
    let Some(expected_context_type) = owner_activation.owner.activation_context_type.as_ref()
    else {
        diagnostics.push(Diagnostic {
            path: module_path.to_path_buf(),
            line: span.line,
            column: span.column,
            message: format!(
                "owner activation `{}` does not use an activation context",
                owner_activation.owner.local_name
            ),
        });
        return;
    };
    let Some(actual_context_type) =
        infer_expr_value_type(workspace, resolved_module, type_scope, scope, context)
    else {
        return;
    };
    let mut semantics = SemanticArena::default();
    let expected_context_id = semantics.type_id_for_hir(
        workspace,
        resolved_module,
        type_scope,
        expected_context_type,
    );
    let actual_context_id =
        semantics.type_id_for_hir(workspace, resolved_module, type_scope, &actual_context_type);
    if actual_context_id != expected_context_id {
        diagnostics.push(Diagnostic {
            path: module_path.to_path_buf(),
            line: span.line,
            column: span.column,
            message: format!(
                "owner activation `{}` expects context `{}`, found `{}`",
                owner_activation.owner.local_name,
                expected_context_type.render(),
                actual_context_type.render()
            ),
        });
    }
}

fn collect_qualified_phrase_param_exprs<'a>(
    symbol: &'a HirSymbol,
    subject: &'a HirExpr,
    args: &'a [arcana_hir::HirPhraseArg],
    qualifier_kind: arcana_hir::HirQualifiedPhraseQualifierKind,
) -> Vec<(&'a arcana_hir::HirParam, &'a HirExpr)> {
    let mut bindings = Vec::new();
    let mut next_positional = 0usize;

    if !matches!(
        qualifier_kind,
        arcana_hir::HirQualifiedPhraseQualifierKind::Call
            | arcana_hir::HirQualifiedPhraseQualifierKind::Weave
            | arcana_hir::HirQualifiedPhraseQualifierKind::Split
    )
        && let Some(param) = symbol.params.first()
    {
        bindings.push((param, subject));
        next_positional = 1;
    }

    for arg in args {
        match arg {
            arcana_hir::HirPhraseArg::Positional(expr) => {
                if let Some(param) = symbol.params.get(next_positional) {
                    bindings.push((param, expr));
                    next_positional += 1;
                }
            }
            arcana_hir::HirPhraseArg::Named { name, value } => {
                if let Some(param) = symbol.params.iter().find(|param| param.name == *name) {
                    bindings.push((param, value));
                }
            }
        }
    }

    bindings
}

fn validate_bare_method_resolution(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    _type_scope: &TypeScope,
    scope: &ValueScope,
    module_path: &Path,
    subject: &HirExpr,
    qualifier_kind: arcana_hir::HirQualifiedPhraseQualifierKind,
    qualifier: &str,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if qualifier_kind != arcana_hir::HirQualifiedPhraseQualifierKind::BareMethod
        || !is_identifier_text(qualifier)
    {
        return;
    }
    let Some(subject_ty) = infer_receiver_expr_type(workspace, resolved_module, scope, subject)
    else {
        return;
    };
    if let Err(message) =
        lookup_method_symbol_for_type(workspace, resolved_module, &subject_ty, qualifier)
    {
        diagnostics.push(Diagnostic {
            path: module_path.to_path_buf(),
            line: span.line,
            column: span.column,
            message,
        });
    }
}

fn validate_borrow_operand_place(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    scope: &ValueScope,
    expr: &HirExpr,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
    mutable: bool,
) {
    let op = if mutable { "&mut" } else { "&" };
    let Some(place_mutability) = expr_place_mutability(expr, scope) else {
        push_type_contract_diagnostic(
            module_path,
            span,
            diagnostics,
            format!("operand of `{op}` must be a local place expression"),
        );
        return;
    };

    if let Some(name) = expr_place_root_local(expr, scope)
        && mutable
        && !scope.is_mutable(name)
    {
        push_type_contract_diagnostic(
            module_path,
            span,
            diagnostics,
            format!("cannot mutably borrow immutable local `{name}`"),
        );
        return;
    }

    if mutable && matches!(place_mutability, PlaceMutability::Immutable) {
        let name = expr_place_root_local(expr, scope).unwrap_or("value");
        push_type_contract_diagnostic(
            module_path,
            span,
            diagnostics,
            format!("cannot mutably borrow immutable local `{name}`"),
        );
        return;
    }

    if let HirExpr::Slice { expr: target, .. } = expr {
        match classify_borrowed_slice_surface(workspace, resolved_module, scope, target) {
            BorrowedSliceSurfaceKind::Array | BorrowedSliceSurfaceKind::EditView => {}
            BorrowedSliceSurfaceKind::ByteEditView if mutable => {}
            BorrowedSliceSurfaceKind::Str | BorrowedSliceSurfaceKind::StrView if mutable => {
                push_type_contract_diagnostic(
                    module_path,
                    span,
                    diagnostics,
                    "string slices are read-only; `&mut x[a..b]` is not allowed".to_string(),
                );
            }
            BorrowedSliceSurfaceKind::ReadView | BorrowedSliceSurfaceKind::ByteView if mutable => {
                push_type_contract_diagnostic(
                    module_path,
                    span,
                    diagnostics,
                    format!("operand of `{op}` is a read-only slice surface"),
                );
            }
            BorrowedSliceSurfaceKind::List => {
                push_type_contract_diagnostic(
                    module_path,
                    span,
                    diagnostics,
                    "borrowed slices require contiguous backing; `List` is not supported"
                        .to_string(),
                );
            }
            BorrowedSliceSurfaceKind::Unsupported => {
                push_type_contract_diagnostic(
                    module_path,
                    span,
                    diagnostics,
                    "borrowed slices require array, string, or view backing".to_string(),
                );
            }
            _ => {}
        }
    }
}

fn validate_direct_local_place_access(
    module_path: &Path,
    scope: &ValueScope,
    state: &BorrowFlowState,
    expr: &HirExpr,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(name) = expr_place_root_local(expr, scope) else {
        return;
    };
    if state.has_moved(name) {
        push_type_contract_diagnostic(
            module_path,
            span,
            diagnostics,
            format!("use of moved local `{name}`"),
        );
        return;
    }
    if state.has_mut_borrow(name) {
        push_type_contract_diagnostic(
            module_path,
            span,
            diagnostics,
            format!("cannot access local `{name}` directly while it is mutably borrowed"),
        );
    }
}

fn validate_place_expr_borrow_flow(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    type_scope: &TypeScope,
    module_path: &Path,
    scope: &ValueScope,
    expr: &HirExpr,
    span: Span,
    state: &mut BorrowFlowState,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match expr {
        HirExpr::Path { .. } => {}
        HirExpr::MemberAccess { expr, .. } => {
            validate_place_expr_borrow_flow(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                expr,
                span,
                state,
                diagnostics,
            );
        }
        HirExpr::Index { expr, index } => {
            validate_place_expr_borrow_flow(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                expr,
                span,
                state,
                diagnostics,
            );
            validate_expr_borrow_flow_inner(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                index,
                span,
                state,
                false,
                diagnostics,
            );
        }
        other => {
            validate_expr_borrow_flow_inner(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                other,
                span,
                state,
                false,
                diagnostics,
            );
        }
    }
}

fn validate_borrow_op_conflict(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    type_scope: &TypeScope,
    module_path: &Path,
    scope: &ValueScope,
    expr: &HirExpr,
    span: Span,
    state: &mut BorrowFlowState,
    diagnostics: &mut Vec<Diagnostic>,
    mutable: bool,
) {
    validate_place_expr_borrow_flow(
        workspace,
        resolved_module,
        type_scope,
        module_path,
        scope,
        expr,
        span,
        state,
        diagnostics,
    );
    let Some(name) = expr_place_root_local(expr, scope) else {
        return;
    };
    if state.has_moved(name) {
        push_type_contract_diagnostic(
            module_path,
            span,
            diagnostics,
            format!("use of moved local `{name}`"),
        );
        return;
    }
    if mutable {
        if state.has_mut_borrow(name) {
            push_type_contract_diagnostic(
                module_path,
                span,
                diagnostics,
                format!("cannot mutably borrow `{name}` while it is already mutably borrowed"),
            );
        } else if state.has_shared_borrow(name) {
            push_type_contract_diagnostic(
                module_path,
                span,
                diagnostics,
                format!("cannot mutably borrow `{name}` while it is already borrowed"),
            );
        }
        state.note_mut_borrow(name);
    } else {
        if state.has_mut_borrow(name) {
            push_type_contract_diagnostic(
                module_path,
                span,
                diagnostics,
                format!("cannot borrow `{name}` while it is mutably borrowed"),
            );
        }
        state.note_shared_borrow(name);
    }
}

fn validate_call_param_mode_flow(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    type_scope: &TypeScope,
    scope: &ValueScope,
    subject: &HirExpr,
    args: &[arcana_hir::HirPhraseArg],
    qualifier_kind: arcana_hir::HirQualifiedPhraseQualifierKind,
    qualifier: &str,
    span: Span,
    state: &mut BorrowFlowState,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(symbol) = resolve_qualified_phrase_target_symbol(
        workspace,
        resolved_module,
        type_scope,
        scope,
        subject,
        qualifier_kind,
        qualifier,
    ) else {
        return;
    };

    for (param, expr) in
        collect_qualified_phrase_param_exprs(symbol, subject, args, qualifier_kind)
    {
        match param.mode {
            Some(arcana_hir::HirParamMode::Read) | None => {
                if let Some(name) = expr_place_root_local(expr, scope) {
                    if state.has_moved(name) {
                        push_type_contract_diagnostic(
                            module_path,
                            span,
                            diagnostics,
                            format!("use of moved local `{name}`"),
                        );
                    } else if state.has_mut_borrow(name) {
                        push_type_contract_diagnostic(
                            module_path,
                            span,
                            diagnostics,
                            format!(
                                "cannot pass local `{name}` to read parameter `{}` while it is mutably borrowed",
                                param.name
                            ),
                        );
                    } else {
                        state.note_shared_borrow(name);
                    }
                }
            }
            Some(arcana_hir::HirParamMode::Edit) => {
                let Some(mutability) = expr_place_mutability(expr, scope) else {
                    push_type_contract_diagnostic(
                        module_path,
                        span,
                        diagnostics,
                        format!(
                            "argument for edit parameter `{}` must be a local place expression",
                            param.name
                        ),
                    );
                    continue;
                };
                let Some(name) = expr_place_root_local(expr, scope) else {
                    continue;
                };
                if state.has_moved(name) {
                    push_type_contract_diagnostic(
                        module_path,
                        span,
                        diagnostics,
                        format!("use of moved local `{name}`"),
                    );
                } else if matches!(mutability, PlaceMutability::Immutable) {
                    push_type_contract_diagnostic(
                        module_path,
                        span,
                        diagnostics,
                        format!(
                            "cannot pass immutable local `{name}` to edit parameter `{}`",
                            param.name
                        ),
                    );
                } else if state.has_mut_borrow(name) {
                    push_type_contract_diagnostic(
                        module_path,
                        span,
                        diagnostics,
                        format!(
                            "cannot pass local `{name}` to edit parameter `{}` while it is already mutably borrowed",
                            param.name
                        ),
                    );
                } else if state.has_shared_borrow(name) {
                    push_type_contract_diagnostic(
                        module_path,
                        span,
                        diagnostics,
                        format!(
                            "cannot pass local `{name}` to edit parameter `{}` while it is already borrowed",
                            param.name
                        ),
                    );
                } else {
                    state.note_mut_borrow(name);
                }
            }
            Some(arcana_hir::HirParamMode::Take) => {
                let Some(name) = expr_place_root_local(expr, scope) else {
                    continue;
                };
                if !scope.ownership_of(name).is_move_only() {
                    continue;
                }
                if state.has_moved(name) {
                    push_type_contract_diagnostic(
                        module_path,
                        span,
                        diagnostics,
                        format!("use of moved local `{name}`"),
                    );
                } else if state.has_any_borrow(name) {
                    push_type_contract_diagnostic(
                        module_path,
                        span,
                        diagnostics,
                        format!("cannot move local `{name}` while it is borrowed"),
                    );
                } else if scope
                    .binding_id_of(name)
                    .is_some_and(|binding_id| state.has_active_cleanup_binding(binding_id))
                {
                    push_type_contract_diagnostic(
                        module_path,
                        span,
                        diagnostics,
                        format!("cleanup subject `{name}` cannot be moved after activation"),
                    );
                } else {
                    state.note_moved(name);
                }
            }
        }
    }
}

fn note_qualified_phrase_moves(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    type_scope: &TypeScope,
    scope: &ValueScope,
    subject: &HirExpr,
    args: &[arcana_hir::HirPhraseArg],
    qualifier_kind: arcana_hir::HirQualifiedPhraseQualifierKind,
    qualifier: &str,
    state: &mut BorrowFlowState,
) {
    let Some(symbol) = resolve_qualified_phrase_target_symbol(
        workspace,
        resolved_module,
        type_scope,
        scope,
        subject,
        qualifier_kind,
        qualifier,
    ) else {
        return;
    };

    for (param, expr) in
        collect_qualified_phrase_param_exprs(symbol, subject, args, qualifier_kind)
    {
        if !matches!(param.mode, Some(arcana_hir::HirParamMode::Take)) {
            continue;
        }
        let Some(name) = expr_place_root_local(expr, scope) else {
            continue;
        };
        if scope.ownership_of(name).is_move_only()
            && !scope
                .binding_id_of(name)
                .is_some_and(|binding_id| state.has_active_cleanup_binding(binding_id))
        {
            state.note_moved(name);
        }
    }
}

fn validate_expr_borrow_flow(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    type_scope: &TypeScope,
    module_path: &Path,
    scope: &ValueScope,
    expr: &HirExpr,
    span: Span,
    state: &BorrowFlowState,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut temp_state = state.clone();
    validate_expr_borrow_flow_inner(
        workspace,
        resolved_module,
        type_scope,
        module_path,
        scope,
        expr,
        span,
        &mut temp_state,
        false,
        diagnostics,
    );
}

fn validate_expr_borrow_flow_inner(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    type_scope: &TypeScope,
    module_path: &Path,
    scope: &ValueScope,
    expr: &HirExpr,
    span: Span,
    state: &mut BorrowFlowState,
    within_place: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match expr {
        HirExpr::Path { segments } => {
            if !within_place && segments.len() == 1 && scope.contains(&segments[0]) {
                validate_direct_local_place_access(
                    module_path,
                    scope,
                    state,
                    expr,
                    span,
                    diagnostics,
                );
            }
        }
        HirExpr::BoolLiteral { .. } | HirExpr::IntLiteral { .. } | HirExpr::StrLiteral { .. } => {}
        HirExpr::Pair { left, right } => {
            validate_expr_borrow_flow_inner(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                left,
                span,
                state,
                false,
                diagnostics,
            );
            validate_expr_borrow_flow_inner(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                right,
                span,
                state,
                false,
                diagnostics,
            );
        }
        HirExpr::CollectionLiteral { items } => {
            for item in items {
                validate_expr_borrow_flow_inner(
                    workspace,
                    resolved_module,
                    type_scope,
                    module_path,
                    scope,
                    item,
                    span,
                    state,
                    false,
                    diagnostics,
                );
            }
        }
        HirExpr::Match { subject, arms } => {
            validate_expr_borrow_flow_inner(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                subject,
                span,
                state,
                false,
                diagnostics,
            );
            for arm in arms {
                let mut arm_state = state.clone();
                validate_expr_borrow_flow_inner(
                    workspace,
                    resolved_module,
                    type_scope,
                    module_path,
                    scope,
                    &arm.value,
                    arm.span,
                    &mut arm_state,
                    false,
                    diagnostics,
                );
            }
        }
        HirExpr::ConstructRegion(region) => {
            validate_expr_borrow_flow_inner(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                &region.target,
                span,
                state,
                false,
                diagnostics,
            );
            if let Some(modifier) = &region.default_modifier
                && let Some(payload) = &modifier.payload
            {
                validate_expr_borrow_flow_inner(
                    workspace,
                    resolved_module,
                    type_scope,
                    module_path,
                    scope,
                    payload,
                    span,
                    state,
                    false,
                    diagnostics,
                );
            }
            for line in &region.lines {
                validate_expr_borrow_flow_inner(
                    workspace,
                    resolved_module,
                    type_scope,
                    module_path,
                    scope,
                    &line.value,
                    line.span,
                    state,
                    false,
                    diagnostics,
                );
                if let Some(modifier) = &line.modifier
                    && let Some(payload) = &modifier.payload
                {
                    validate_expr_borrow_flow_inner(
                        workspace,
                        resolved_module,
                        type_scope,
                        module_path,
                        scope,
                        payload,
                        line.span,
                        state,
                        false,
                        diagnostics,
                    );
                }
            }
        }
        HirExpr::RecordRegion(region) => {
            validate_expr_borrow_flow_inner(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                &region.target,
                span,
                state,
                false,
                diagnostics,
            );
            if let Some(base) = &region.base {
                validate_expr_borrow_flow_inner(
                    workspace,
                    resolved_module,
                    type_scope,
                    module_path,
                    scope,
                    base,
                    span,
                    state,
                    false,
                    diagnostics,
                );
            }
            if let Some(modifier) = &region.default_modifier
                && let Some(payload) = &modifier.payload
            {
                validate_expr_borrow_flow_inner(
                    workspace,
                    resolved_module,
                    type_scope,
                    module_path,
                    scope,
                    payload,
                    span,
                    state,
                    false,
                    diagnostics,
                );
            }
            for line in &region.lines {
                validate_expr_borrow_flow_inner(
                    workspace,
                    resolved_module,
                    type_scope,
                    module_path,
                    scope,
                    &line.value,
                    line.span,
                    state,
                    false,
                    diagnostics,
                );
                if let Some(modifier) = &line.modifier
                    && let Some(payload) = &modifier.payload
                {
                    validate_expr_borrow_flow_inner(
                        workspace,
                        resolved_module,
                        type_scope,
                        module_path,
                        scope,
                        payload,
                        line.span,
                        state,
                        false,
                        diagnostics,
                    );
                }
            }
        }
        HirExpr::Chain { steps, .. } => {
            for step in steps {
                validate_expr_borrow_flow_inner(
                    workspace,
                    resolved_module,
                    type_scope,
                    module_path,
                    scope,
                    &step.stage,
                    span,
                    state,
                    false,
                    diagnostics,
                );
                for arg in &step.bind_args {
                    validate_expr_borrow_flow_inner(
                        workspace,
                        resolved_module,
                        type_scope,
                        module_path,
                        scope,
                        arg,
                        span,
                        state,
                        false,
                        diagnostics,
                    );
                }
            }
        }
        HirExpr::MemoryPhrase {
            arena,
            init_args,
            constructor,
            attached,
            ..
        } => {
            validate_expr_borrow_flow_inner(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                arena,
                span,
                state,
                false,
                diagnostics,
            );
            for arg in init_args {
                match arg {
                    arcana_hir::HirPhraseArg::Positional(expr)
                    | arcana_hir::HirPhraseArg::Named { value: expr, .. } => {
                        validate_expr_borrow_flow_inner(
                            workspace,
                            resolved_module,
                            type_scope,
                            module_path,
                            scope,
                            expr,
                            span,
                            state,
                            false,
                            diagnostics,
                        );
                    }
                }
            }
            validate_expr_borrow_flow_inner(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                constructor,
                span,
                state,
                false,
                diagnostics,
            );
            for attachment in attached {
                match attachment {
                    HirHeaderAttachment::Named { value, span, .. }
                    | HirHeaderAttachment::Chain {
                        expr: value, span, ..
                    } => validate_expr_borrow_flow_inner(
                        workspace,
                        resolved_module,
                        type_scope,
                        module_path,
                        scope,
                        value,
                        *span,
                        state,
                        false,
                        diagnostics,
                    ),
                }
            }
        }
        HirExpr::GenericApply { expr, .. } | HirExpr::Await { expr } => {
            validate_expr_borrow_flow_inner(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                expr,
                span,
                state,
                false,
                diagnostics,
            );
        }
        HirExpr::QualifiedPhrase {
            subject,
            args,
            qualifier_kind,
            qualifier,
            attached,
            ..
        } => {
            validate_expr_borrow_flow_inner(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                subject,
                span,
                state,
                false,
                diagnostics,
            );
            for arg in args {
                match arg {
                    arcana_hir::HirPhraseArg::Positional(expr)
                    | arcana_hir::HirPhraseArg::Named { value: expr, .. } => {
                        validate_expr_borrow_flow_inner(
                            workspace,
                            resolved_module,
                            type_scope,
                            module_path,
                            scope,
                            expr,
                            span,
                            state,
                            false,
                            diagnostics,
                        );
                    }
                }
            }
            for attachment in attached {
                match attachment {
                    HirHeaderAttachment::Named { value, span, .. }
                    | HirHeaderAttachment::Chain {
                        expr: value, span, ..
                    } => validate_expr_borrow_flow_inner(
                        workspace,
                        resolved_module,
                        type_scope,
                        module_path,
                        scope,
                        value,
                        *span,
                        state,
                        false,
                        diagnostics,
                    ),
                }
            }
            validate_call_param_mode_flow(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                subject,
                args,
                *qualifier_kind,
                qualifier,
                span,
                state,
                diagnostics,
            );
        }
        HirExpr::Unary { op, expr } => match op {
            HirUnaryOp::BorrowRead => validate_borrow_op_conflict(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                expr,
                span,
                state,
                diagnostics,
                false,
            ),
            HirUnaryOp::BorrowMut => validate_borrow_op_conflict(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                expr,
                span,
                state,
                diagnostics,
                true,
            ),
            HirUnaryOp::Deref | HirUnaryOp::Neg | HirUnaryOp::Not | HirUnaryOp::BitNot => {
                validate_expr_borrow_flow_inner(
                    workspace,
                    resolved_module,
                    type_scope,
                    module_path,
                    scope,
                    expr,
                    span,
                    state,
                    false,
                    diagnostics,
                );
            }
            HirUnaryOp::Weave | HirUnaryOp::Split => {
                validate_expr_borrow_flow_inner(
                    workspace,
                    resolved_module,
                    type_scope,
                    module_path,
                    scope,
                    expr,
                    span,
                    state,
                    false,
                    diagnostics,
                );
            }
        },
        HirExpr::Binary { left, right, .. } => {
            validate_expr_borrow_flow_inner(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                left,
                span,
                state,
                false,
                diagnostics,
            );
            validate_expr_borrow_flow_inner(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                right,
                span,
                state,
                false,
                diagnostics,
            );
        }
        HirExpr::MemberAccess { expr: target, .. } => {
            if !within_place {
                validate_direct_local_place_access(
                    module_path,
                    scope,
                    state,
                    expr,
                    span,
                    diagnostics,
                );
            }
            validate_expr_borrow_flow_inner(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                target,
                span,
                state,
                true,
                diagnostics,
            );
        }
        HirExpr::Index {
            expr: target,
            index,
        } => {
            if !within_place {
                validate_direct_local_place_access(
                    module_path,
                    scope,
                    state,
                    expr,
                    span,
                    diagnostics,
                );
            }
            validate_expr_borrow_flow_inner(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                target,
                span,
                state,
                true,
                diagnostics,
            );
            validate_expr_borrow_flow_inner(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                index,
                span,
                state,
                false,
                diagnostics,
            );
        }
        HirExpr::Slice {
            expr: target,
            start,
            end,
            ..
        } => {
            if !within_place {
                validate_direct_local_place_access(
                    module_path,
                    scope,
                    state,
                    expr,
                    span,
                    diagnostics,
                );
            }
            validate_expr_borrow_flow_inner(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                target,
                span,
                state,
                true,
                diagnostics,
            );
            if let Some(start) = start {
                validate_expr_borrow_flow_inner(
                    workspace,
                    resolved_module,
                    type_scope,
                    module_path,
                    scope,
                    start,
                    span,
                    state,
                    false,
                    diagnostics,
                );
            }
            if let Some(end) = end {
                validate_expr_borrow_flow_inner(
                    workspace,
                    resolved_module,
                    type_scope,
                    module_path,
                    scope,
                    end,
                    span,
                    state,
                    false,
                    diagnostics,
                );
            }
        }
        HirExpr::Range { start, end, .. } => {
            if let Some(start) = start {
                validate_expr_borrow_flow_inner(
                    workspace,
                    resolved_module,
                    type_scope,
                    module_path,
                    scope,
                    start,
                    span,
                    state,
                    false,
                    diagnostics,
                );
            }
            if let Some(end) = end {
                validate_expr_borrow_flow_inner(
                    workspace,
                    resolved_module,
                    type_scope,
                    module_path,
                    scope,
                    end,
                    span,
                    state,
                    false,
                    diagnostics,
                );
            }
        }
    }
}

fn collect_expr_local_borrows(
    expr: &HirExpr,
    scope: &ValueScope,
    borrows: &mut Vec<(String, bool)>,
) {
    match expr {
        HirExpr::Unary { op, expr } => {
            if matches!(op, HirUnaryOp::BorrowRead | HirUnaryOp::BorrowMut)
                && let Some(name) = expr_place_root_local(expr, scope)
            {
                borrows.push((name.to_string(), matches!(op, HirUnaryOp::BorrowMut)));
            }
            collect_expr_local_borrows(expr, scope, borrows);
        }
        HirExpr::Pair { left, right } | HirExpr::Binary { left, right, .. } => {
            collect_expr_local_borrows(left, scope, borrows);
            collect_expr_local_borrows(right, scope, borrows);
        }
        HirExpr::CollectionLiteral { items } => {
            for item in items {
                collect_expr_local_borrows(item, scope, borrows);
            }
        }
        HirExpr::Match { subject, arms } => {
            collect_expr_local_borrows(subject, scope, borrows);
            for arm in arms {
                collect_expr_local_borrows(&arm.value, scope, borrows);
            }
        }
        HirExpr::ConstructRegion(region) => {
            collect_expr_local_borrows(&region.target, scope, borrows);
            if let Some(modifier) = &region.default_modifier
                && let Some(payload) = &modifier.payload
            {
                collect_expr_local_borrows(payload, scope, borrows);
            }
            for line in &region.lines {
                collect_expr_local_borrows(&line.value, scope, borrows);
                if let Some(modifier) = &line.modifier
                    && let Some(payload) = &modifier.payload
                {
                    collect_expr_local_borrows(payload, scope, borrows);
                }
            }
        }
        HirExpr::RecordRegion(region) => {
            collect_expr_local_borrows(&region.target, scope, borrows);
            if let Some(base) = &region.base {
                collect_expr_local_borrows(base, scope, borrows);
            }
            if let Some(modifier) = &region.default_modifier
                && let Some(payload) = &modifier.payload
            {
                collect_expr_local_borrows(payload, scope, borrows);
            }
            for line in &region.lines {
                collect_expr_local_borrows(&line.value, scope, borrows);
                if let Some(modifier) = &line.modifier
                    && let Some(payload) = &modifier.payload
                {
                    collect_expr_local_borrows(payload, scope, borrows);
                }
            }
        }
        HirExpr::Chain { steps, .. } => {
            for step in steps {
                collect_expr_local_borrows(&step.stage, scope, borrows);
                for arg in &step.bind_args {
                    collect_expr_local_borrows(arg, scope, borrows);
                }
            }
        }
        HirExpr::MemoryPhrase {
            arena,
            init_args,
            constructor,
            attached,
            ..
        } => {
            collect_expr_local_borrows(arena, scope, borrows);
            for arg in init_args {
                match arg {
                    arcana_hir::HirPhraseArg::Positional(expr)
                    | arcana_hir::HirPhraseArg::Named { value: expr, .. } => {
                        collect_expr_local_borrows(expr, scope, borrows);
                    }
                }
            }
            collect_expr_local_borrows(constructor, scope, borrows);
            for attachment in attached {
                match attachment {
                    HirHeaderAttachment::Named { value, .. }
                    | HirHeaderAttachment::Chain { expr: value, .. } => {
                        collect_expr_local_borrows(value, scope, borrows);
                    }
                }
            }
        }
        HirExpr::GenericApply { expr, .. }
        | HirExpr::Await { expr }
        | HirExpr::MemberAccess { expr, .. } => collect_expr_local_borrows(expr, scope, borrows),
        HirExpr::QualifiedPhrase {
            subject,
            args,
            attached,
            ..
        } => {
            collect_expr_local_borrows(subject, scope, borrows);
            for arg in args {
                match arg {
                    arcana_hir::HirPhraseArg::Positional(expr)
                    | arcana_hir::HirPhraseArg::Named { value: expr, .. } => {
                        collect_expr_local_borrows(expr, scope, borrows);
                    }
                }
            }
            for attachment in attached {
                match attachment {
                    HirHeaderAttachment::Named { value, .. }
                    | HirHeaderAttachment::Chain { expr: value, .. } => {
                        collect_expr_local_borrows(value, scope, borrows);
                    }
                }
            }
        }
        HirExpr::Index { expr, index } => {
            collect_expr_local_borrows(expr, scope, borrows);
            collect_expr_local_borrows(index, scope, borrows);
        }
        HirExpr::Slice {
            expr, start, end, ..
        } => {
            collect_expr_local_borrows(expr, scope, borrows);
            if let Some(start) = start {
                collect_expr_local_borrows(start, scope, borrows);
            }
            if let Some(end) = end {
                collect_expr_local_borrows(end, scope, borrows);
            }
        }
        HirExpr::Range { start, end, .. } => {
            if let Some(start) = start {
                collect_expr_local_borrows(start, scope, borrows);
            }
            if let Some(end) = end {
                collect_expr_local_borrows(end, scope, borrows);
            }
        }
        HirExpr::Path { .. }
        | HirExpr::BoolLiteral { .. }
        | HirExpr::IntLiteral { .. }
        | HirExpr::StrLiteral { .. } => {}
    }
}

fn note_escaping_expr_borrows(state: &mut BorrowFlowState, expr: &HirExpr, scope: &ValueScope) {
    let mut borrows = Vec::new();
    collect_expr_local_borrows(expr, scope, &mut borrows);
    for (name, mutable) in borrows {
        if mutable {
            state.note_mut_borrow(&name);
        } else {
            state.note_shared_borrow(&name);
        }
    }
}

fn note_expr_moves(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    type_scope: &TypeScope,
    scope: &ValueScope,
    expr: &HirExpr,
    state: &mut BorrowFlowState,
) {
    match expr {
        HirExpr::QualifiedPhrase {
            subject,
            args,
            qualifier_kind,
            qualifier,
            ..
        } => {
            note_qualified_phrase_moves(
                workspace,
                resolved_module,
                type_scope,
                scope,
                subject,
                args,
                *qualifier_kind,
                qualifier,
                state,
            );
            note_expr_moves(
                workspace,
                resolved_module,
                type_scope,
                scope,
                subject,
                state,
            );
            for arg in args {
                match arg {
                    arcana_hir::HirPhraseArg::Positional(expr)
                    | arcana_hir::HirPhraseArg::Named { value: expr, .. } => {
                        note_expr_moves(workspace, resolved_module, type_scope, scope, expr, state);
                    }
                }
            }
        }
        HirExpr::GenericApply { expr, .. }
        | HirExpr::Await { expr }
        | HirExpr::MemberAccess { expr, .. } => {
            note_expr_moves(workspace, resolved_module, type_scope, scope, expr, state);
        }
        HirExpr::Unary { expr, .. } => {
            note_expr_moves(workspace, resolved_module, type_scope, scope, expr, state);
        }
        HirExpr::Pair { left, right } | HirExpr::Binary { left, right, .. } => {
            note_expr_moves(workspace, resolved_module, type_scope, scope, left, state);
            note_expr_moves(workspace, resolved_module, type_scope, scope, right, state);
        }
        HirExpr::CollectionLiteral { items } => {
            for item in items {
                note_expr_moves(workspace, resolved_module, type_scope, scope, item, state);
            }
        }
        HirExpr::Match { subject, arms } => {
            note_expr_moves(
                workspace,
                resolved_module,
                type_scope,
                scope,
                subject,
                state,
            );
            for arm in arms {
                note_expr_moves(
                    workspace,
                    resolved_module,
                    type_scope,
                    scope,
                    &arm.value,
                    state,
                );
            }
        }
        HirExpr::ConstructRegion(region) => {
            note_expr_moves(
                workspace,
                resolved_module,
                type_scope,
                scope,
                &region.target,
                state,
            );
            if let Some(modifier) = &region.default_modifier
                && let Some(payload) = &modifier.payload
            {
                note_expr_moves(
                    workspace,
                    resolved_module,
                    type_scope,
                    scope,
                    payload,
                    state,
                );
            }
            for line in &region.lines {
                note_expr_moves(
                    workspace,
                    resolved_module,
                    type_scope,
                    scope,
                    &line.value,
                    state,
                );
                if let Some(modifier) = &line.modifier
                    && let Some(payload) = &modifier.payload
                {
                    note_expr_moves(
                        workspace,
                        resolved_module,
                        type_scope,
                        scope,
                        payload,
                        state,
                    );
                }
            }
        }
        HirExpr::RecordRegion(region) => {
            note_expr_moves(
                workspace,
                resolved_module,
                type_scope,
                scope,
                &region.target,
                state,
            );
            if let Some(base) = &region.base {
                note_expr_moves(workspace, resolved_module, type_scope, scope, base, state);
            }
            if let Some(modifier) = &region.default_modifier
                && let Some(payload) = &modifier.payload
            {
                note_expr_moves(
                    workspace,
                    resolved_module,
                    type_scope,
                    scope,
                    payload,
                    state,
                );
            }
            for line in &region.lines {
                note_expr_moves(
                    workspace,
                    resolved_module,
                    type_scope,
                    scope,
                    &line.value,
                    state,
                );
                if let Some(modifier) = &line.modifier
                    && let Some(payload) = &modifier.payload
                {
                    note_expr_moves(
                        workspace,
                        resolved_module,
                        type_scope,
                        scope,
                        payload,
                        state,
                    );
                }
            }
        }
        HirExpr::Chain { steps, .. } => {
            for step in steps {
                note_expr_moves(
                    workspace,
                    resolved_module,
                    type_scope,
                    scope,
                    &step.stage,
                    state,
                );
                for arg in &step.bind_args {
                    note_expr_moves(workspace, resolved_module, type_scope, scope, arg, state);
                }
            }
        }
        HirExpr::MemoryPhrase {
            arena,
            init_args,
            constructor,
            attached,
            ..
        } => {
            note_expr_moves(workspace, resolved_module, type_scope, scope, arena, state);
            for arg in init_args {
                match arg {
                    arcana_hir::HirPhraseArg::Positional(expr)
                    | arcana_hir::HirPhraseArg::Named { value: expr, .. } => {
                        note_expr_moves(workspace, resolved_module, type_scope, scope, expr, state);
                    }
                }
            }
            note_expr_moves(
                workspace,
                resolved_module,
                type_scope,
                scope,
                constructor,
                state,
            );
            for attachment in attached {
                match attachment {
                    HirHeaderAttachment::Named { value, .. }
                    | HirHeaderAttachment::Chain { expr: value, .. } => {
                        note_expr_moves(
                            workspace,
                            resolved_module,
                            type_scope,
                            scope,
                            value,
                            state,
                        );
                    }
                }
            }
        }
        HirExpr::Index { expr, index } => {
            note_expr_moves(workspace, resolved_module, type_scope, scope, expr, state);
            note_expr_moves(workspace, resolved_module, type_scope, scope, index, state);
        }
        HirExpr::Slice {
            expr, start, end, ..
        } => {
            note_expr_moves(workspace, resolved_module, type_scope, scope, expr, state);
            if let Some(start) = start {
                note_expr_moves(workspace, resolved_module, type_scope, scope, start, state);
            }
            if let Some(end) = end {
                note_expr_moves(workspace, resolved_module, type_scope, scope, end, state);
            }
        }
        HirExpr::Range { start, end, .. } => {
            if let Some(start) = start {
                note_expr_moves(workspace, resolved_module, type_scope, scope, start, state);
            }
            if let Some(end) = end {
                note_expr_moves(workspace, resolved_module, type_scope, scope, end, state);
            }
        }
        HirExpr::Path { .. }
        | HirExpr::BoolLiteral { .. }
        | HirExpr::IntLiteral { .. }
        | HirExpr::StrLiteral { .. } => {}
    }
}

fn collect_returned_local_borrows(
    expr: &HirExpr,
    scope: &ValueScope,
    roots: &mut BTreeSet<String>,
) {
    match expr {
        HirExpr::Unary { op, expr } => {
            if matches!(op, HirUnaryOp::BorrowRead | HirUnaryOp::BorrowMut)
                && let Some(name) = expr_place_root_local(expr, scope)
            {
                roots.insert(name.to_string());
            }
            collect_returned_local_borrows(expr, scope, roots);
        }
        HirExpr::Pair { left, right } | HirExpr::Binary { left, right, .. } => {
            collect_returned_local_borrows(left, scope, roots);
            collect_returned_local_borrows(right, scope, roots);
        }
        HirExpr::CollectionLiteral { items } => {
            for item in items {
                collect_returned_local_borrows(item, scope, roots);
            }
        }
        HirExpr::Match { subject, arms } => {
            collect_returned_local_borrows(subject, scope, roots);
            for arm in arms {
                collect_returned_local_borrows(&arm.value, scope, roots);
            }
        }
        HirExpr::ConstructRegion(region) => {
            collect_returned_local_borrows(&region.target, scope, roots);
            if let Some(modifier) = &region.default_modifier
                && let Some(payload) = &modifier.payload
            {
                collect_returned_local_borrows(payload, scope, roots);
            }
            for line in &region.lines {
                collect_returned_local_borrows(&line.value, scope, roots);
                if let Some(modifier) = &line.modifier
                    && let Some(payload) = &modifier.payload
                {
                    collect_returned_local_borrows(payload, scope, roots);
                }
            }
        }
        HirExpr::RecordRegion(region) => {
            collect_returned_local_borrows(&region.target, scope, roots);
            if let Some(base) = &region.base {
                collect_returned_local_borrows(base, scope, roots);
            }
            if let Some(modifier) = &region.default_modifier
                && let Some(payload) = &modifier.payload
            {
                collect_returned_local_borrows(payload, scope, roots);
            }
            for line in &region.lines {
                collect_returned_local_borrows(&line.value, scope, roots);
                if let Some(modifier) = &line.modifier
                    && let Some(payload) = &modifier.payload
                {
                    collect_returned_local_borrows(payload, scope, roots);
                }
            }
        }
        HirExpr::Chain { steps, .. } => {
            for step in steps {
                collect_returned_local_borrows(&step.stage, scope, roots);
                for arg in &step.bind_args {
                    collect_returned_local_borrows(arg, scope, roots);
                }
            }
        }
        HirExpr::MemoryPhrase {
            arena,
            init_args,
            constructor,
            attached,
            ..
        } => {
            collect_returned_local_borrows(arena, scope, roots);
            for arg in init_args {
                match arg {
                    arcana_hir::HirPhraseArg::Positional(expr)
                    | arcana_hir::HirPhraseArg::Named { value: expr, .. } => {
                        collect_returned_local_borrows(expr, scope, roots);
                    }
                }
            }
            collect_returned_local_borrows(constructor, scope, roots);
            for attachment in attached {
                match attachment {
                    HirHeaderAttachment::Named { value, .. }
                    | HirHeaderAttachment::Chain { expr: value, .. } => {
                        collect_returned_local_borrows(value, scope, roots);
                    }
                }
            }
        }
        HirExpr::GenericApply { expr, .. }
        | HirExpr::Await { expr }
        | HirExpr::MemberAccess { expr, .. } => collect_returned_local_borrows(expr, scope, roots),
        HirExpr::QualifiedPhrase {
            subject,
            args,
            attached,
            ..
        } => {
            collect_returned_local_borrows(subject, scope, roots);
            for arg in args {
                match arg {
                    arcana_hir::HirPhraseArg::Positional(expr)
                    | arcana_hir::HirPhraseArg::Named { value: expr, .. } => {
                        collect_returned_local_borrows(expr, scope, roots);
                    }
                }
            }
            for attachment in attached {
                match attachment {
                    HirHeaderAttachment::Named { value, .. }
                    | HirHeaderAttachment::Chain { expr: value, .. } => {
                        collect_returned_local_borrows(value, scope, roots);
                    }
                }
            }
        }
        HirExpr::Index { expr, index } => {
            collect_returned_local_borrows(expr, scope, roots);
            collect_returned_local_borrows(index, scope, roots);
        }
        HirExpr::Slice {
            expr, start, end, ..
        } => {
            collect_returned_local_borrows(expr, scope, roots);
            if let Some(start) = start {
                collect_returned_local_borrows(start, scope, roots);
            }
            if let Some(end) = end {
                collect_returned_local_borrows(end, scope, roots);
            }
        }
        HirExpr::Range { start, end, .. } => {
            if let Some(start) = start {
                collect_returned_local_borrows(start, scope, roots);
            }
            if let Some(end) = end {
                collect_returned_local_borrows(end, scope, roots);
            }
        }
        HirExpr::Path { .. }
        | HirExpr::BoolLiteral { .. }
        | HirExpr::IntLiteral { .. }
        | HirExpr::StrLiteral { .. } => {}
    }
}

fn validate_return_borrow_ties(
    module_path: &Path,
    scope: &ValueScope,
    expr: &HirExpr,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut roots = BTreeSet::new();
    collect_returned_local_borrows(expr, scope, &mut roots);
    for root in roots {
        if scope.is_param(&root) {
            continue;
        }
        push_type_contract_diagnostic(
            module_path,
            span,
            diagnostics,
            format!(
                "returned reference must be tied to input lifetimes; local `{root}` does not live long enough"
            ),
        );
    }
}

fn validate_opaque_constructor_semantics(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    type_scope: &TypeScope,
    scope: &ValueScope,
    subject: &HirExpr,
    qualifier_kind: &arcana_hir::HirQualifiedPhraseQualifierKind,
    qualifier: &str,
    qualifier_type_args: &[HirType],
    args: &[arcana_hir::HirPhraseArg],
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match qualifier_kind {
        arcana_hir::HirQualifiedPhraseQualifierKind::Must => {
            if !args.is_empty() {
                diagnostics.push(Diagnostic {
                    path: module_path.to_path_buf(),
                    line: span.line,
                    column: span.column,
                    message: "`:: must` does not accept arguments".to_string(),
                });
            }
            if let Some(subject_ty) = infer_expr_value_type(workspace, resolved_module, type_scope, scope, subject)
                && type_option_payload(&subject_ty).is_none()
                && type_result_payloads(&subject_ty)
                    .is_none_or(|(_, err)| err.render() != "Str")
            {
                diagnostics.push(Diagnostic {
                    path: module_path.to_path_buf(),
                    line: span.line,
                    column: span.column,
                    message: "`:: must` expects `Option[T]` or `Result[T, Str]`".to_string(),
                });
            }
            return;
        }
        arcana_hir::HirQualifiedPhraseQualifierKind::Fallback => {
            let positional = args
                .iter()
                .filter(|arg| matches!(arg, arcana_hir::HirPhraseArg::Positional(_)))
                .count();
            let named = args
                .iter()
                .any(|arg| matches!(arg, arcana_hir::HirPhraseArg::Named { .. }));
            if positional != 1 || named {
                diagnostics.push(Diagnostic {
                    path: module_path.to_path_buf(),
                    line: span.line,
                    column: span.column,
                    message:
                        "`:: fallback` expects exactly one positional fallback argument"
                            .to_string(),
                });
            }
            if let Some(subject_ty) = infer_expr_value_type(workspace, resolved_module, type_scope, scope, subject)
                && type_option_payload(&subject_ty).is_none()
                && type_result_payloads(&subject_ty)
                    .is_none_or(|(_, err)| err.render() != "Str")
            {
                diagnostics.push(Diagnostic {
                    path: module_path.to_path_buf(),
                    line: span.line,
                    column: span.column,
                    message: "`:: fallback` expects `Option[T]` or `Result[T, Str]`".to_string(),
                });
            }
            return;
        }
        arcana_hir::HirQualifiedPhraseQualifierKind::Await => {
            if !args.is_empty() {
                diagnostics.push(Diagnostic {
                    path: module_path.to_path_buf(),
                    line: span.line,
                    column: span.column,
                    message: "`:: await` does not accept arguments".to_string(),
                });
            }
            return;
        }
        arcana_hir::HirQualifiedPhraseQualifierKind::Call
        | arcana_hir::HirQualifiedPhraseQualifierKind::Weave
        | arcana_hir::HirQualifiedPhraseQualifierKind::Split => {}
        _ => return,
    }
    if !qualifier_type_args.is_empty()
        && !matches!(
            qualifier_kind,
            arcana_hir::HirQualifiedPhraseQualifierKind::Call
                | arcana_hir::HirQualifiedPhraseQualifierKind::Weave
                | arcana_hir::HirQualifiedPhraseQualifierKind::Split
                | arcana_hir::HirQualifiedPhraseQualifierKind::BareMethod
                | arcana_hir::HirQualifiedPhraseQualifierKind::NamedPath
        )
    {
        return;
    }
    let Some(symbol) = resolve_qualified_phrase_target_symbol(
        workspace,
        resolved_module,
        type_scope,
        scope,
        subject,
        *qualifier_kind,
        qualifier,
    ) else {
        return;
    };
    if symbol.kind != HirSymbolKind::OpaqueType {
        return;
    }
    diagnostics.push(Diagnostic {
        path: module_path.to_path_buf(),
        line: span.line,
        column: span.column,
        message: format!("opaque type `{}` is not constructible", symbol.name),
    });
}

pub fn check_sources<'a, I>(sources: I) -> Result<CheckSummary, String>
where
    I: IntoIterator<Item = &'a str>,
{
    let mut summary = CheckSummary::default();
    for (index, source) in sources.into_iter().enumerate() {
        let hir = lower_module_text(format!("memory.module.{index}"), source)?;
        summary.module_count += 1;
        summary.non_empty_lines += hir.non_empty_line_count;
        summary.directive_count += hir.directives.len();
        summary.symbol_count += hir.symbols.len();
    }
    Ok(summary)
}

pub fn check_path(path: &Path) -> Result<CheckSummary, String> {
    let metadata =
        fs::metadata(path).map_err(|err| format!("failed to read `{}`: {err}", path.display()))?;
    if metadata.is_file() {
        return check_file(path);
    }
    if !metadata.is_dir() {
        return Err(format!("`{}` is not a file or directory", path.display()));
    }

    Ok(check_workspace_path(path)?.into_summary())
}

pub fn load_workspace_hir(path: &Path) -> Result<HirWorkspaceSummary, String> {
    let root_dir = canonicalize_workspace_dir(path)?;
    load_package_workspace_hir(&root_dir)
}

pub fn check_workspace_path(path: &Path) -> Result<CheckedWorkspace, String> {
    let root_dir = canonicalize_workspace_dir(path)?;
    let workspace = load_package_workspace_hir(&root_dir)?;
    validate_packages(workspace)
}

pub fn check_workspace_graph(graph: &WorkspaceGraph) -> Result<CheckedWorkspace, String> {
    let workspace = load_package_workspace_hir_from_graph(&graph.root_dir, graph)?;
    validate_packages(workspace)
}

pub fn compute_member_fingerprints_for_checked_workspace(
    graph: &WorkspaceGraph,
    checked: &CheckedWorkspace,
) -> Result<WorkspaceFingerprints, String> {
    api_fingerprint::compute_member_fingerprints_for_checked_workspace(graph, checked)
}

pub fn compute_member_fingerprints(
    graph: &WorkspaceGraph,
) -> Result<WorkspaceFingerprints, String> {
    let checked = check_workspace_graph(graph)?;
    api_fingerprint::compute_member_fingerprints_for_checked_workspace(graph, &checked)
}

pub fn lower_to_hir(summary: &CheckSummary) -> HirModule {
    HirModule {
        symbol_count: summary.symbol_count.max(summary.module_count),
        item_count: summary.non_empty_lines + summary.directive_count,
    }
}

fn check_file(path: &Path) -> Result<CheckSummary, String> {
    let source = fs::read_to_string(path)
        .map_err(|err| format!("failed to read `{}`: {err}", path.display()))?;
    let hir = lower_module_text(path.display().to_string(), &source)
        .map_err(|err| format!("{}: {err}", path.display()))?;
    Ok(CheckSummary {
        package_count: 0,
        module_count: 1,
        non_empty_lines: hir.non_empty_line_count,
        directive_count: hir.directives.len(),
        symbol_count: hir.symbols.len(),
        warning_count: 0,
    })
}

fn canonicalize_workspace_dir(path: &Path) -> Result<PathBuf, String> {
    let metadata =
        fs::metadata(path).map_err(|err| format!("failed to read `{}`: {err}", path.display()))?;
    if !metadata.is_dir() {
        return Err(format!(
            "workspace HIR requires a grimoire or workspace directory, got `{}`",
            path.display()
        ));
    }

    let root_dir = fs::canonicalize(path)
        .map_err(|err| format!("failed to open `{}`: {err}", path.display()))?;
    let manifest_path = root_dir.join("book.toml");
    if !manifest_path.is_file() {
        return Err(format!(
            "`{}` does not contain a `book.toml` manifest",
            root_dir.display()
        ));
    }

    Ok(root_dir)
}

fn render_resolution_errors(
    workspace: &HirWorkspaceSummary,
    errors: Vec<arcana_hir::HirResolutionError>,
) -> String {
    errors
        .into_iter()
        .map(|error| {
            let package = workspace.package(&error.package_name);
            let path = package
                .and_then(|package| package.module_path(&error.source_module_id))
                .cloned()
                .unwrap_or_else(|| {
                    package
                        .map(|package| package.root_dir.join("src").join("unknown.arc"))
                        .unwrap_or_else(|| PathBuf::from("unknown.arc"))
                });
            Diagnostic {
                path,
                line: error.span.line,
                column: error.span.column,
                message: error.message,
            }
            .render()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn validate_packages(workspace: HirWorkspaceSummary) -> Result<CheckedWorkspace, String> {
    let (workspace, transform_warnings) = apply_executable_foreword_transforms(workspace)?;
    let (workspace, metadata_warnings) = apply_executable_foreword_metadata(workspace)?;
    let workspace = populate_basic_foreword_registrations(workspace);
    let mut summary = summarize_workspace(&workspace);
    let resolved_workspace = resolve_workspace(&workspace)
        .map_err(|errors| render_resolution_errors(&workspace, errors))?;
    let mut validation = validate_hir_semantics(&workspace, &resolved_workspace);
    let mut warnings = transform_warnings;
    warnings.extend(metadata_warnings);
    warnings.append(&mut validation.warnings);
    summary.warning_count = warnings.len();

    if validation.errors.is_empty() {
        return Ok(CheckedWorkspace {
            summary,
            workspace,
            resolved_workspace,
            warnings,
            discovered_tests: validation.discovered_tests,
            foreword_catalog: validation.foreword_catalog,
            foreword_index: validation.foreword_index,
            foreword_registrations: validation.foreword_registrations,
        });
    }

    Err(render_diagnostics(validation.errors))
}

fn summarize_workspace(workspace: &HirWorkspaceSummary) -> CheckSummary {
    let mut summary = CheckSummary {
        package_count: workspace.package_count(),
        ..CheckSummary::default()
    };

    for package in workspace.packages.values() {
        for module in &package.summary.modules {
            summary.module_count += 1;
            summary.non_empty_lines += module.non_empty_line_count;
            summary.directive_count += module.directives.len();
            summary.symbol_count += module.symbols.len();
        }
    }

    summary
}

fn render_diagnostics(mut diagnostics: Vec<Diagnostic>) -> String {
    diagnostics.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| left.line.cmp(&right.line))
            .then_with(|| left.column.cmp(&right.column))
            .then_with(|| left.message.cmp(&right.message))
    });
    diagnostics
        .into_iter()
        .map(|diagnostic| diagnostic.render())
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_foreword_args(app: &arcana_hir::HirForewordApp) -> Vec<String> {
    app.args
        .iter()
        .map(|arg| match &arg.name {
            Some(name) => format!("{name}={}", arg.typed_value.render()),
            None => arg.typed_value.render(),
        })
        .collect()
}

fn render_hir_foreword_args(args: &[arcana_hir::HirForewordArg]) -> Vec<String> {
    args.iter()
        .map(|arg| match &arg.name {
            Some(name) => format!("{name}={}", arg.typed_value.render()),
            None => arg.typed_value.render(),
        })
        .collect()
}

fn lower_adapter_payload_value(
    value: &arcana_hir::HirForewordArgValue,
) -> AdapterPayloadValueSnapshot {
    match value {
        arcana_hir::HirForewordArgValue::Raw(value) => {
            AdapterPayloadValueSnapshot::Raw(value.clone())
        }
        arcana_hir::HirForewordArgValue::Bool(value) => AdapterPayloadValueSnapshot::Bool(*value),
        arcana_hir::HirForewordArgValue::Int(value) => AdapterPayloadValueSnapshot::Int(*value),
        arcana_hir::HirForewordArgValue::Str(value) => {
            AdapterPayloadValueSnapshot::Str(value.clone())
        }
        arcana_hir::HirForewordArgValue::Symbol(value) => {
            AdapterPayloadValueSnapshot::Symbol(value.clone())
        }
        arcana_hir::HirForewordArgValue::Path(value) => {
            AdapterPayloadValueSnapshot::Path(value.clone())
        }
    }
}

fn lower_adapter_payload_args(
    args: &[arcana_hir::HirForewordArg],
) -> Vec<AdapterPayloadArgSnapshot> {
    args.iter()
        .map(|arg| AdapterPayloadArgSnapshot {
            name: arg.name.clone(),
            rendered: arg.typed_value.render(),
            value: lower_adapter_payload_value(&arg.typed_value),
        })
        .collect()
}

fn lower_generated_by(generated_by: &arcana_hir::HirGeneratedByForeword) -> ForewordGeneratedBy {
    ForewordGeneratedBy {
        applied_name: generated_by.applied_name.clone(),
        resolved_name: generated_by.resolved_name.clone(),
        provider_package_id: generated_by.provider_package_id.clone(),
        owner_kind: generated_by.owner_kind.clone(),
        owner_path: generated_by.owner_path.clone(),
        args: render_hir_foreword_args(&generated_by.args),
    }
}

fn lower_response_args(args: &[AdapterResponseArg]) -> Vec<arcana_hir::HirForewordArg> {
    args.iter()
        .map(|arg| arcana_hir::HirForewordArg {
            name: arg.name.clone(),
            value: arg.value.clone(),
            typed_value: classify_hir_foreword_arg_value(&arg.value),
        })
        .collect()
}

fn classify_hir_foreword_arg_value(source: &str) -> arcana_hir::HirForewordArgValue {
    let trimmed = source.trim();
    if let Some(unquoted) = trimmed
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    {
        return arcana_hir::HirForewordArgValue::Str(unquoted.to_string());
    }
    if trimmed == "true" {
        return arcana_hir::HirForewordArgValue::Bool(true);
    }
    if trimmed == "false" {
        return arcana_hir::HirForewordArgValue::Bool(false);
    }
    if let Ok(value) = trimmed.parse::<i64>() {
        return arcana_hir::HirForewordArgValue::Int(value);
    }
    if let Some(path) = split_simple_path(trimmed) {
        if path.len() == 1 {
            return arcana_hir::HirForewordArgValue::Symbol(path[0].clone());
        }
        return arcana_hir::HirForewordArgValue::Path(path);
    }
    arcana_hir::HirForewordArgValue::Raw(trimmed.to_string())
}

fn parse_adapter_retention(text: &str) -> Option<arcana_hir::HirForewordRetention> {
    match text.trim() {
        "compile" => Some(arcana_hir::HirForewordRetention::Compile),
        "tooling" => Some(arcana_hir::HirForewordRetention::Tooling),
        "runtime" => Some(arcana_hir::HirForewordRetention::Runtime),
        _ => None,
    }
}

fn collect_emitted_metadata(
    module_path: &Path,
    app: &arcana_hir::HirForewordApp,
    owner_public: bool,
    generated_by: &arcana_hir::HirGeneratedByForeword,
    descriptors: &[AdapterEmittedMetadataDescriptor],
    errors: &mut Vec<Diagnostic>,
) -> Vec<arcana_hir::HirEmittedForewordMetadata> {
    let mut emitted = Vec::new();
    for descriptor in descriptors {
        let qualified_name = descriptor
            .qualified_name
            .clone()
            .unwrap_or_else(|| generated_by.resolved_name.clone());
        if split_simple_path(&qualified_name).is_none() {
            errors.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: app.span.line,
                column: app.span.column,
                message: format!(
                    "foreword adapter emitted metadata uses invalid qualified_name `{qualified_name}`"
                ),
            });
            continue;
        }
        let target_kind = descriptor
            .target_kind
            .clone()
            .unwrap_or_else(|| generated_by.owner_kind.clone());
        let target_path = descriptor
            .target_path
            .clone()
            .unwrap_or_else(|| generated_by.owner_path.clone());
        if target_kind.trim().is_empty() || target_path.trim().is_empty() {
            errors.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: app.span.line,
                column: app.span.column,
                message:
                    "foreword adapter emitted metadata must provide a non-empty target_kind and target_path"
                        .to_string(),
            });
            continue;
        }
        let retention = match descriptor.retention.as_deref() {
            Some(value) => match parse_adapter_retention(value) {
                Some(retention) => retention,
                None => {
                    errors.push(Diagnostic {
                        path: module_path.to_path_buf(),
                        line: app.span.line,
                        column: app.span.column,
                        message: format!(
                            "foreword adapter emitted metadata uses unsupported retention `{value}`"
                        ),
                    });
                    continue;
                }
            },
            None => generated_by.retention,
        };
        emitted.push(arcana_hir::HirEmittedForewordMetadata {
            qualified_name,
            target_kind,
            target_path,
            retention,
            args: lower_response_args(&descriptor.args),
            public: descriptor.public.unwrap_or(owner_public),
            generated_by: generated_by.clone(),
        });
    }
    emitted
}

fn collect_registration_rows(
    module_path: &Path,
    app: &arcana_hir::HirForewordApp,
    owner_public: bool,
    generated_by: &arcana_hir::HirGeneratedByForeword,
    rows: &[AdapterRegistrationRowDescriptor],
    errors: &mut Vec<Diagnostic>,
) -> Vec<arcana_hir::HirForewordRegistrationRow> {
    let mut registrations = Vec::new();
    for row in rows {
        if split_simple_path(&row.namespace).is_none() {
            errors.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: app.span.line,
                column: app.span.column,
                message: format!(
                    "foreword adapter registration row uses invalid namespace `{}`",
                    row.namespace
                ),
            });
            continue;
        }
        if row.key.trim().is_empty() {
            errors.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: app.span.line,
                column: app.span.column,
                message: "foreword adapter registration row key must not be empty".to_string(),
            });
            continue;
        }
        let target_kind = row
            .target_kind
            .clone()
            .unwrap_or_else(|| generated_by.owner_kind.clone());
        let target_path = row
            .target_path
            .clone()
            .unwrap_or_else(|| generated_by.owner_path.clone());
        if target_kind.trim().is_empty() || target_path.trim().is_empty() {
            errors.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: app.span.line,
                column: app.span.column,
                message:
                    "foreword adapter registration rows must provide a non-empty target_kind and target_path"
                        .to_string(),
            });
            continue;
        }
        registrations.push(arcana_hir::HirForewordRegistrationRow {
            namespace: row.namespace.clone(),
            key: row.key.clone(),
            value: row.value.clone(),
            target_kind,
            target_path,
            public: row.public.unwrap_or(owner_public),
            generated_by: generated_by.clone(),
        });
    }
    registrations
}

const BUILTIN_FOREWORD_PROVIDER_PACKAGE_ID: &str = "arcana.builtin";

fn builtin_foreword_catalog_entry(
    exposed_name: &str,
    action: &str,
    targets: &[&str],
) -> ForewordCatalogEntry {
    ForewordCatalogEntry {
        provider_package_id: BUILTIN_FOREWORD_PROVIDER_PACKAGE_ID.to_string(),
        exposed_name: exposed_name.to_string(),
        qualified_name: exposed_name.to_string(),
        tier: "builtin".to_string(),
        visibility: "public".to_string(),
        action: action.to_string(),
        retention: "compile".to_string(),
        targets: targets.iter().map(|target| (*target).to_string()).collect(),
        diagnostic_namespace: None,
        handler: None,
    }
}

fn builtin_foreword_catalog_entries() -> Vec<ForewordCatalogEntry> {
    vec![
        builtin_foreword_catalog_entry(
            "deprecated",
            "metadata",
            &[
                "fn",
                "record",
                "obj",
                "owner",
                "enum",
                "opaque_type",
                "trait",
                "trait_method",
                "impl_method",
                "const",
                "field",
                "param",
            ],
        ),
        builtin_foreword_catalog_entry(
            "only",
            "metadata",
            &[
                "import",
                "reexport",
                "use",
                "trait",
                "behavior",
                "system",
                "fn",
                "record",
                "obj",
                "owner",
                "enum",
                "opaque_type",
                "trait_method",
                "impl_method",
                "const",
                "field",
                "param",
            ],
        ),
        builtin_foreword_catalog_entry("test", "metadata", &["fn"]),
        builtin_foreword_catalog_entry(
            "allow",
            "metadata",
            &[
                "import",
                "reexport",
                "use",
                "trait",
                "behavior",
                "system",
                "fn",
                "record",
                "obj",
                "owner",
                "enum",
                "opaque_type",
                "trait_method",
                "impl_method",
                "const",
                "field",
                "param",
            ],
        ),
        builtin_foreword_catalog_entry(
            "deny",
            "metadata",
            &[
                "import",
                "reexport",
                "use",
                "trait",
                "behavior",
                "system",
                "fn",
                "record",
                "obj",
                "owner",
                "enum",
                "opaque_type",
                "trait_method",
                "impl_method",
                "const",
                "field",
                "param",
            ],
        ),
        builtin_foreword_catalog_entry(
            "inline",
            "metadata",
            &["fn", "trait_method", "impl_method"],
        ),
        builtin_foreword_catalog_entry("cold", "metadata", &["fn", "trait_method", "impl_method"]),
        builtin_foreword_catalog_entry("boundary", "metadata", &["fn", "impl_method"]),
        builtin_foreword_catalog_entry(
            "stage",
            "metadata",
            &["fn", "trait_method", "impl_method", "behavior", "system"],
        ),
        builtin_foreword_catalog_entry("chain", "metadata", &["statement.chain"]),
    ]
}

fn builtin_foreword_targets(targets: &[arcana_hir::HirForewordDefinitionTarget]) -> Vec<String> {
    targets
        .iter()
        .map(|target| target.as_str().to_string())
        .collect()
}

fn builtin_foreword_export(
    name: &str,
    targets: Vec<arcana_hir::HirForewordDefinitionTarget>,
) -> ResolvedForewordExport {
    ResolvedForewordExport {
        kind: ResolvedForewordExportKind::Builtin,
        provider_package_id: BUILTIN_FOREWORD_PROVIDER_PACKAGE_ID.to_string(),
        exposed_package_id: BUILTIN_FOREWORD_PROVIDER_PACKAGE_ID.to_string(),
        exposed_name: vec![name.to_string()],
        definition: arcana_hir::HirForewordDefinition {
            qualified_name: vec![name.to_string()],
            tier: arcana_hir::HirForewordTier::Basic,
            visibility: arcana_hir::HirForewordVisibility::Public,
            phase: arcana_hir::HirForewordPhase::Frontend,
            action: arcana_hir::HirForewordAction::Metadata,
            targets,
            retention: arcana_hir::HirForewordRetention::Compile,
            payload: Vec::new(),
            repeatable: false,
            conflicts: Vec::new(),
            diagnostic_namespace: None,
            handler: None,
            span: Span { line: 0, column: 0 },
        },
        handler: None,
        public: true,
    }
}

fn builtin_foreword_exports() -> Vec<ResolvedForewordExport> {
    use arcana_hir::HirForewordDefinitionTarget as Target;

    vec![
        builtin_foreword_export(
            "deprecated",
            vec![
                Target::Function,
                Target::Record,
                Target::Object,
                Target::Owner,
                Target::Enum,
                Target::OpaqueType,
                Target::Trait,
                Target::TraitMethod,
                Target::ImplMethod,
                Target::Const,
                Target::Field,
                Target::Param,
            ],
        ),
        builtin_foreword_export(
            "only",
            vec![
                Target::Import,
                Target::Reexport,
                Target::Use,
                Target::Trait,
                Target::Behavior,
                Target::System,
                Target::Function,
                Target::Record,
                Target::Object,
                Target::Owner,
                Target::Enum,
                Target::OpaqueType,
                Target::TraitMethod,
                Target::ImplMethod,
                Target::Const,
                Target::Field,
                Target::Param,
            ],
        ),
        builtin_foreword_export("test", vec![Target::Function]),
        builtin_foreword_export(
            "allow",
            vec![
                Target::Import,
                Target::Reexport,
                Target::Use,
                Target::Trait,
                Target::Behavior,
                Target::System,
                Target::Function,
                Target::Record,
                Target::Object,
                Target::Owner,
                Target::Enum,
                Target::OpaqueType,
                Target::TraitMethod,
                Target::ImplMethod,
                Target::Const,
                Target::Field,
                Target::Param,
            ],
        ),
        builtin_foreword_export(
            "deny",
            vec![
                Target::Import,
                Target::Reexport,
                Target::Use,
                Target::Trait,
                Target::Behavior,
                Target::System,
                Target::Function,
                Target::Record,
                Target::Object,
                Target::Owner,
                Target::Enum,
                Target::OpaqueType,
                Target::TraitMethod,
                Target::ImplMethod,
                Target::Const,
                Target::Field,
                Target::Param,
            ],
        ),
        builtin_foreword_export(
            "inline",
            vec![Target::Function, Target::TraitMethod, Target::ImplMethod],
        ),
        builtin_foreword_export(
            "cold",
            vec![Target::Function, Target::TraitMethod, Target::ImplMethod],
        ),
        builtin_foreword_export("boundary", vec![Target::Function, Target::ImplMethod]),
        builtin_foreword_export(
            "stage",
            vec![
                Target::Function,
                Target::TraitMethod,
                Target::ImplMethod,
                Target::Behavior,
                Target::System,
            ],
        ),
    ]
}

fn build_adapter_payload_schema(
    definition: &arcana_hir::HirForewordDefinition,
) -> Vec<AdapterPayloadFieldSnapshot> {
    definition
        .payload
        .iter()
        .map(|field| AdapterPayloadFieldSnapshot {
            name: field.name.clone(),
            optional: field.optional,
            ty: field.ty.as_str().to_string(),
        })
        .collect()
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn hash_json_hex<T: Serialize>(value: &T) -> String {
    let bytes = serde_json::to_vec(value).expect("cache key inputs should serialize");
    sha256_hex(&bytes)
}

fn digest_path_contents(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    Some(sha256_hex(&bytes))
}

#[cfg(windows)]
fn external_process_path(path: &Path) -> PathBuf {
    let rendered = path.as_os_str().to_string_lossy();
    if let Some(stripped) = rendered.strip_prefix(r"\\?\UNC\") {
        PathBuf::from(format!(r"\\{stripped}"))
    } else if let Some(stripped) = rendered.strip_prefix(r"\\?\") {
        PathBuf::from(stripped)
    } else {
        path.to_path_buf()
    }
}

#[cfg(not(windows))]
fn external_process_path(path: &Path) -> PathBuf {
    path.to_path_buf()
}

fn external_process_path_string(path: &Path) -> String {
    external_process_path(path).to_string_lossy().to_string()
}

fn resolve_adapter_runner_program(provider_package: &HirWorkspacePackage, runner: &str) -> String {
    let runner_path = Path::new(runner);
    if runner_path.is_absolute() || runner_path.components().count() == 1 {
        external_process_path_string(runner_path)
    } else {
        external_process_path_string(&provider_package.root_dir.join(runner_path))
    }
}

fn materialized_foreword_adapter_root(
    provider_package: &HirWorkspacePackage,
    product: &arcana_hir::HirForewordAdapterProduct,
) -> PathBuf {
    let product_path = provider_package.root_dir.join(&product.path);
    let runner = product
        .runner
        .as_ref()
        .map(|runner| resolve_adapter_runner_program(provider_package, runner));
    let seed = hash_json_hex(&(
        &provider_package.package_id,
        &product.name,
        &product.path,
        &product.args,
        runner.as_ref(),
        digest_path_contents(&product_path),
        runner.as_ref().and_then(|program| {
            let path = Path::new(program);
            (path.is_absolute() || program.contains(std::path::MAIN_SEPARATOR))
                .then(|| digest_path_contents(path))
                .flatten()
        }),
    ));
    provider_package
        .root_dir
        .join(".arcana")
        .join("foreword-products")
        .join(seed)
}

fn copy_adapter_artifact_if_needed(source: &Path, target: &Path) -> Result<(), String> {
    if source == target {
        return Ok(());
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create foreword adapter artifact directory `{}`: {err}",
                parent.display()
            )
        })?;
    }
    let source_bytes = fs::read(source).map_err(|err| {
        format!(
            "failed to read foreword adapter artifact `{}`: {err}",
            source.display()
        )
    })?;
    let target_matches = fs::read(target)
        .ok()
        .is_some_and(|existing| existing == source_bytes);
    if !target_matches {
        fs::write(target, &source_bytes).map_err(|err| {
            format!(
                "failed to write foreword adapter artifact `{}`: {err}",
                target.display()
            )
        })?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(source)
            .map_err(|err| {
                format!(
                    "failed to read foreword adapter artifact metadata `{}`: {err}",
                    source.display()
                )
            })?
            .permissions();
        perms.set_mode(perms.mode() | 0o111);
        fs::set_permissions(target, perms).map_err(|err| {
            format!(
                "failed to update foreword adapter artifact permissions `{}`: {err}",
                target.display()
            )
        })?;
    }
    Ok(())
}

fn copy_adapter_sidecars_if_present(source: &Path, target: &Path) -> Result<(), String> {
    let Some(parent) = source.parent() else {
        return Ok(());
    };
    let Some(stem) = source.file_stem() else {
        return Ok(());
    };
    for entry in fs::read_dir(parent).map_err(|err| {
        format!(
            "failed to enumerate foreword adapter sidecars in `{}`: {err}",
            parent.display()
        )
    })? {
        let entry = entry.map_err(|err| {
            format!(
                "failed to inspect foreword adapter sidecar in `{}`: {err}",
                parent.display()
            )
        })?;
        let sidecar = entry.path();
        if sidecar == source || sidecar.file_stem() != Some(stem) || !sidecar.is_file() {
            continue;
        }
        let sidecar_target = target
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(entry.file_name());
        copy_adapter_artifact_if_needed(&sidecar, &sidecar_target)?;
    }
    Ok(())
}

fn materialize_foreword_adapter_artifact(
    provider_package: &HirWorkspacePackage,
    product: &arcana_hir::HirForewordAdapterProduct,
) -> Result<MaterializedForewordAdapterArtifact, String> {
    let root = materialized_foreword_adapter_root(provider_package, product);
    let source_product_path = provider_package.root_dir.join(&product.path);
    let product_file_name = source_product_path
        .file_name()
        .ok_or_else(|| {
            format!(
                "foreword adapter `{}` has invalid product path `{}`",
                product.name, product.path
            )
        })?
        .to_owned();
    let staged_product_path = root.join(product_file_name);
    copy_adapter_artifact_if_needed(&source_product_path, &staged_product_path)?;
    copy_adapter_sidecars_if_present(&source_product_path, &staged_product_path)?;

    let (runner_program, runner_digest) = if let Some(runner) = &product.runner {
        let resolved_runner = resolve_adapter_runner_program(provider_package, runner);
        let runner_path = Path::new(&resolved_runner);
        if runner_path.is_absolute() || resolved_runner.contains(std::path::MAIN_SEPARATOR) {
            let runner_file_name = runner_path.file_name().ok_or_else(|| {
                format!(
                    "foreword adapter `{}` has invalid runner path `{resolved_runner}`",
                    product.name
                )
            })?;
            let staged_runner_path = root.join(runner_file_name);
            copy_adapter_artifact_if_needed(runner_path, &staged_runner_path)?;
            copy_adapter_sidecars_if_present(runner_path, &staged_runner_path)?;
            (
                Some(external_process_path_string(&staged_runner_path)),
                digest_path_contents(&staged_runner_path),
            )
        } else {
            (Some(resolved_runner), None)
        }
    } else {
        (None, None)
    };

    Ok(MaterializedForewordAdapterArtifact {
        product_program: external_process_path_string(&staged_product_path),
        product_arg_path: external_process_path_string(&staged_product_path),
        runner_program,
        args: product.args.clone(),
        product_digest: digest_path_contents(&staged_product_path),
        runner_digest,
    })
}

fn build_adapter_artifact_identity(
    provider_package: &HirWorkspacePackage,
    product: &arcana_hir::HirForewordAdapterProduct,
) -> AdapterArtifactIdentity {
    let materialized = materialize_foreword_adapter_artifact(provider_package, product)
        .unwrap_or_else(|_| MaterializedForewordAdapterArtifact {
            product_program: external_process_path_string(
                &provider_package.root_dir.join(&product.path),
            ),
            product_arg_path: external_process_path_string(
                &provider_package.root_dir.join(&product.path),
            ),
            runner_program: product
                .runner
                .as_ref()
                .map(|runner| resolve_adapter_runner_program(provider_package, runner)),
            args: product.args.clone(),
            product_digest: digest_path_contents(&provider_package.root_dir.join(&product.path)),
            runner_digest: product.runner.as_ref().and_then(|runner| {
                let program = resolve_adapter_runner_program(provider_package, runner);
                let path = Path::new(&program);
                (path.is_absolute() || program.contains(std::path::MAIN_SEPARATOR))
                    .then(|| digest_path_contents(path))
                    .flatten()
            }),
        });
    AdapterArtifactIdentity {
        product_name: product.name.clone(),
        product_path: materialized.product_arg_path,
        runner: materialized.runner_program,
        args: materialized.args,
        product_digest: materialized.product_digest,
        runner_digest: materialized.runner_digest,
    }
}

fn build_foreword_definition_schema_hash(definition: &arcana_hir::HirForewordDefinition) -> String {
    #[derive(Serialize)]
    struct ForewordDefinitionSchema<'a> {
        qualified_name: &'a [String],
        tier: &'static str,
        visibility: &'static str,
        phase: &'static str,
        action: &'static str,
        targets: Vec<&'static str>,
        retention: &'static str,
        payload: Vec<(&'a str, bool, &'static str)>,
        repeatable: bool,
        conflicts: &'a [Vec<String>],
        diagnostic_namespace: &'a Option<String>,
        handler: &'a Option<Vec<String>>,
    }

    hash_json_hex(&ForewordDefinitionSchema {
        qualified_name: &definition.qualified_name,
        tier: definition.tier.as_str(),
        visibility: definition.visibility.as_str(),
        phase: definition.phase.as_str(),
        action: definition.action.as_str(),
        targets: definition
            .targets
            .iter()
            .map(|target| target.as_str())
            .collect(),
        retention: definition.retention.as_str(),
        payload: definition
            .payload
            .iter()
            .map(|field| (field.name.as_str(), field.optional, field.ty.as_str()))
            .collect(),
        repeatable: definition.repeatable,
        conflicts: &definition.conflicts,
        diagnostic_namespace: &definition.diagnostic_namespace,
        handler: &definition.handler,
    })
}

fn build_foreword_adapter_cache_key(
    package: &HirWorkspacePackage,
    export: &ResolvedForewordExport,
    handler: &arcana_hir::HirForewordHandler,
    product: &arcana_hir::HirForewordAdapterProduct,
    target: &AdapterTargetSnapshot,
    rendered_args: &[String],
    visible_forewords: &[AdapterCatalogEntry],
    dependency_opt_in_enabled: bool,
    artifact: &AdapterArtifactIdentity,
) -> String {
    #[derive(Serialize)]
    struct ForewordAdapterCacheSeed<'a> {
        protocol_version: &'a str,
        toolchain_version: &'a str,
        definition_schema_hash: String,
        handler_binding: String,
        adapter_artifact_identity: &'a AdapterArtifactIdentity,
        visible_dependency_foreword_registry: &'a [AdapterCatalogEntry],
        dependency_opt_in_enabled: bool,
        consumer_package_id: &'a str,
        provider_package_id: &'a str,
        exposed_package_id: &'a str,
        applied_name: String,
        resolved_name: String,
        target_kind: &'a str,
        target_path: &'a str,
        target_public: bool,
        args: &'a [String],
    }

    hash_json_hex(&ForewordAdapterCacheSeed {
        protocol_version: FOREWORD_ADAPTER_PROTOCOL_VERSION,
        toolchain_version: env!("CARGO_PKG_VERSION"),
        definition_schema_hash: build_foreword_definition_schema_hash(&export.definition),
        handler_binding: format!(
            "{}:{}:{}:{}",
            handler.qualified_name.join("."),
            handler.protocol,
            product.name,
            handler.entry
        ),
        adapter_artifact_identity: artifact,
        visible_dependency_foreword_registry: visible_forewords,
        dependency_opt_in_enabled,
        consumer_package_id: &package.package_id,
        provider_package_id: &export.provider_package_id,
        exposed_package_id: &export.exposed_package_id,
        applied_name: export.exposed_name.join("."),
        resolved_name: export.definition.qualified_name.join("."),
        target_kind: &target.kind,
        target_path: &target.path,
        target_public: target.public,
        args: rendered_args,
    })
}

fn build_foreword_registry(workspace: &HirWorkspaceSummary) -> ForewordRegistry {
    let mut registry = ForewordRegistry::default();
    registry.catalog.extend(builtin_foreword_catalog_entries());
    for export in builtin_foreword_exports() {
        registry.exports.insert(
            (
                export.exposed_package_id.clone(),
                export.exposed_name.join("."),
            ),
            export,
        );
    }
    let mut local_defs = BTreeMap::<(String, String), arcana_hir::HirForewordDefinition>::new();
    let mut local_handlers = BTreeMap::<(String, String), arcana_hir::HirForewordHandler>::new();

    for package in workspace.packages.values() {
        let package_name = &package.summary.package_name;
        for module in &package.summary.modules {
            let module_path = package
                .module_path(&module.module_id)
                .cloned()
                .unwrap_or_else(|| package.root_dir.join("src").join("unknown.arc"));
            for definition in &module.foreword_definitions {
                if definition.qualified_name.len() < 2
                    || definition.qualified_name[0] != *package_name
                {
                    registry.errors.push(Diagnostic {
                        path: module_path.clone(),
                        line: definition.span.line,
                        column: definition.span.column,
                        message: format!(
                            "foreword `{}` must use the owning package root `{}`",
                            definition.qualified_name.join("."),
                            package_name
                        ),
                    });
                    continue;
                }
                let tail = definition.qualified_name[1..].join(".");
                if local_defs
                    .insert(
                        (package.package_id.clone(), tail.clone()),
                        definition.clone(),
                    )
                    .is_some()
                {
                    registry.errors.push(Diagnostic {
                        path: module_path.clone(),
                        line: definition.span.line,
                        column: definition.span.column,
                        message: format!(
                            "duplicate foreword definition `{}`",
                            definition.qualified_name.join(".")
                        ),
                    });
                }
            }
            for handler in &module.foreword_handlers {
                if handler.qualified_name.len() < 2 || handler.qualified_name[0] != *package_name {
                    registry.errors.push(Diagnostic {
                        path: module_path.clone(),
                        line: handler.span.line,
                        column: handler.span.column,
                        message: format!(
                            "foreword handler `{}` must use the owning package root `{}`",
                            handler.qualified_name.join("."),
                            package_name
                        ),
                    });
                    continue;
                }
                let tail = handler.qualified_name[1..].join(".");
                if local_handlers
                    .insert((package.package_id.clone(), tail.clone()), handler.clone())
                    .is_some()
                {
                    registry.errors.push(Diagnostic {
                        path: module_path.clone(),
                        line: handler.span.line,
                        column: handler.span.column,
                        message: format!(
                            "duplicate foreword handler `{}`",
                            handler.qualified_name.join(".")
                        ),
                    });
                }
            }
        }
    }

    for package in workspace.packages.values() {
        let package_name = &package.summary.package_name;
        for ((provider_package_id, tail), definition) in &local_defs {
            if provider_package_id != &package.package_id {
                continue;
            }
            let handler = definition.handler.as_ref().and_then(|path| {
                if path.len() < 2 {
                    return None;
                }
                local_handlers
                    .get(&(provider_package_id.clone(), path[1..].join(".")))
                    .cloned()
            });
            let exposed_name = definition.qualified_name.clone();
            let public = definition.visibility == arcana_hir::HirForewordVisibility::Public;
            registry.catalog.push(ForewordCatalogEntry {
                provider_package_id: provider_package_id.clone(),
                exposed_name: exposed_name.join("."),
                qualified_name: definition.qualified_name.join("."),
                tier: definition.tier.as_str().to_string(),
                visibility: definition.visibility.as_str().to_string(),
                action: definition.action.as_str().to_string(),
                retention: definition.retention.as_str().to_string(),
                targets: definition
                    .targets
                    .iter()
                    .map(|target| target.as_str().to_string())
                    .collect(),
                diagnostic_namespace: definition.diagnostic_namespace.clone(),
                handler: handler.as_ref().map(|item| item.qualified_name.join(".")),
            });
            registry.exports.insert(
                (package.package_id.clone(), tail.clone()),
                ResolvedForewordExport {
                    kind: ResolvedForewordExportKind::User,
                    provider_package_id: provider_package_id.clone(),
                    exposed_package_id: package.package_id.clone(),
                    exposed_name,
                    definition: definition.clone(),
                    handler,
                    public,
                },
            );
        }
        for module in &package.summary.modules {
            let module_path = package
                .module_path(&module.module_id)
                .cloned()
                .unwrap_or_else(|| package.root_dir.join("src").join("unknown.arc"));
            for alias in &module.foreword_aliases {
                if alias.alias_name.len() < 2 || alias.alias_name[0] != *package_name {
                    registry.errors.push(Diagnostic {
                        path: module_path.clone(),
                        line: alias.span.line,
                        column: alias.span.column,
                        message: format!(
                            "foreword alias `{}` must use the owning package root `{}`",
                            alias.alias_name.join("."),
                            package_name
                        ),
                    });
                    continue;
                }
                if alias.source_name.len() < 2 {
                    registry.errors.push(Diagnostic {
                        path: module_path.clone(),
                        line: alias.span.line,
                        column: alias.span.column,
                        message: "foreword aliases must reference a qualified source foreword"
                            .to_string(),
                    });
                    continue;
                }
                let provider_package_id = if alias.source_name[0] == *package_name {
                    package.package_id.clone()
                } else if let Some(dep_id) = package.direct_dep_ids.get(&alias.source_name[0]) {
                    dep_id.clone()
                } else if let Some((_, dep_id)) = package
                    .direct_dep_packages
                    .iter()
                    .find(|(_, dep_name)| **dep_name == alias.source_name[0])
                    .and_then(|(alias_name, _)| {
                        package
                            .direct_dep_ids
                            .get(alias_name)
                            .map(|dep_id| (alias_name, dep_id))
                    })
                {
                    dep_id.clone()
                } else {
                    registry.errors.push(Diagnostic {
                        path: module_path.clone(),
                        line: alias.span.line,
                        column: alias.span.column,
                        message: format!(
                            "foreword alias source package `{}` is not visible here",
                            alias.source_name[0]
                        ),
                    });
                    continue;
                };
                let source_tail = alias.source_name[1..].join(".");
                let Some(definition) = local_defs
                    .get(&(provider_package_id.clone(), source_tail.clone()))
                    .cloned()
                else {
                    registry.errors.push(Diagnostic {
                        path: module_path.clone(),
                        line: alias.span.line,
                        column: alias.span.column,
                        message: format!(
                            "foreword alias source `{}` is not defined",
                            alias.source_name.join(".")
                        ),
                    });
                    continue;
                };
                if provider_package_id != package.package_id
                    && definition.visibility != arcana_hir::HirForewordVisibility::Public
                {
                    registry.errors.push(Diagnostic {
                        path: module_path.clone(),
                        line: alias.span.line,
                        column: alias.span.column,
                        message: format!(
                            "foreword alias source `{}` is not public",
                            alias.source_name.join(".")
                        ),
                    });
                    continue;
                }
                let handler = definition.handler.as_ref().and_then(|path| {
                    if path.len() < 2 {
                        return None;
                    }
                    local_handlers
                        .get(&(provider_package_id.clone(), path[1..].join(".")))
                        .cloned()
                });
                let exposed_tail = alias.alias_name[1..].join(".");
                let public = alias.kind == arcana_hir::HirForewordAliasKind::Reexport;
                if registry
                    .exports
                    .insert(
                        (package.package_id.clone(), exposed_tail),
                        ResolvedForewordExport {
                            kind: ResolvedForewordExportKind::User,
                            provider_package_id: provider_package_id.clone(),
                            exposed_package_id: package.package_id.clone(),
                            exposed_name: alias.alias_name.clone(),
                            definition: definition.clone(),
                            handler: handler.clone(),
                            public,
                        },
                    )
                    .is_some()
                {
                    registry.errors.push(Diagnostic {
                        path: module_path.clone(),
                        line: alias.span.line,
                        column: alias.span.column,
                        message: format!(
                            "duplicate exposed foreword name `{}`",
                            alias.alias_name.join(".")
                        ),
                    });
                    continue;
                }
                registry.catalog.push(ForewordCatalogEntry {
                    provider_package_id,
                    exposed_name: alias.alias_name.join("."),
                    qualified_name: definition.qualified_name.join("."),
                    tier: definition.tier.as_str().to_string(),
                    visibility: if public { "public" } else { "package" }.to_string(),
                    action: definition.action.as_str().to_string(),
                    retention: definition.retention.as_str().to_string(),
                    targets: definition
                        .targets
                        .iter()
                        .map(|target| target.as_str().to_string())
                        .collect(),
                    diagnostic_namespace: definition.diagnostic_namespace.clone(),
                    handler: handler.as_ref().map(|item| item.qualified_name.join(".")),
                });
            }
        }
    }

    registry.catalog.sort_by(|left, right| {
        left.exposed_name
            .cmp(&right.exposed_name)
            .then_with(|| left.provider_package_id.cmp(&right.provider_package_id))
    });
    registry
}

fn resolve_foreword_export<'a>(
    package: &HirWorkspacePackage,
    app: &arcana_hir::HirForewordApp,
    registry: &'a ForewordRegistry,
) -> Option<&'a ResolvedForewordExport> {
    if app.path.len() == 1 {
        return registry.exports.get(&(
            BUILTIN_FOREWORD_PROVIDER_PACKAGE_ID.to_string(),
            app.name.clone(),
        ));
    }
    if app.path.len() < 2 {
        return None;
    }
    let package_id = if app.path[0] == package.summary.package_name {
        package.package_id.clone()
    } else if let Some(dep_id) = package.direct_dep_ids.get(&app.path[0]) {
        dep_id.clone()
    } else if let Some((alias, _)) = package
        .direct_dep_packages
        .iter()
        .find(|(_, dep_name)| **dep_name == app.path[0])
    {
        package.direct_dep_ids.get(alias)?.clone()
    } else {
        return None;
    };
    let export = registry
        .exports
        .get(&(package_id.clone(), app.path[1..].join(".")))?;
    if package_id == package.package_id || export.public {
        Some(export)
    } else {
        None
    }
}

fn resolve_user_foreword_export<'a>(
    package: &HirWorkspacePackage,
    app: &arcana_hir::HirForewordApp,
    registry: &'a ForewordRegistry,
) -> Option<&'a ResolvedForewordExport> {
    resolve_foreword_export(package, app, registry).filter(|export| !export.is_builtin())
}

const FOREWORD_ADAPTER_PROTOCOL_VERSION: &str = "arcana-foreword-stdio-v1";

fn render_foreword_app_text(app: &arcana_hir::HirForewordApp) -> String {
    if app.args.is_empty() {
        format!("#{}", app.path.join("."))
    } else {
        format!(
            "#{}[{}]",
            app.path.join("."),
            render_foreword_args(app).join(", ")
        )
    }
}

fn build_adapter_field_snapshot(field: &arcana_hir::HirField) -> AdapterFieldSnapshot {
    AdapterFieldSnapshot {
        name: field.name.clone(),
        ty: field.ty.to_string(),
        forewords: field
            .forewords
            .iter()
            .map(render_foreword_app_text)
            .collect(),
    }
}

fn build_adapter_param_snapshot(param: &arcana_hir::HirParam) -> AdapterParamSnapshot {
    AdapterParamSnapshot {
        mode: param.mode.map(|mode| mode.as_str().to_string()),
        name: param.name.clone(),
        ty: param.ty.to_string(),
        forewords: param
            .forewords
            .iter()
            .map(render_foreword_app_text)
            .collect(),
    }
}

fn build_adapter_symbol_snapshot(symbol: &HirSymbol) -> AdapterSymbolSnapshot {
    let (fields, methods, variants, assoc_types) = match &symbol.body {
        HirSymbolBody::Record { fields } => (
            fields.iter().map(build_adapter_field_snapshot).collect(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        ),
        HirSymbolBody::Object { fields, methods } => (
            fields.iter().map(build_adapter_field_snapshot).collect(),
            methods.iter().map(build_adapter_symbol_snapshot).collect(),
            Vec::new(),
            Vec::new(),
        ),
        HirSymbolBody::Enum { variants } => (
            Vec::new(),
            Vec::new(),
            variants
                .iter()
                .map(|variant| variant.name.clone())
                .collect(),
            Vec::new(),
        ),
        HirSymbolBody::Trait {
            assoc_types,
            methods,
        } => (
            Vec::new(),
            methods.iter().map(build_adapter_symbol_snapshot).collect(),
            Vec::new(),
            assoc_types.iter().map(|assoc| assoc.name.clone()).collect(),
        ),
        HirSymbolBody::Owner { objects, exits, .. } => (
            Vec::new(),
            Vec::new(),
            objects
                .iter()
                .map(|object| {
                    format!(
                        "object:{}:{}",
                        object.local_name,
                        object.type_path.join(".")
                    )
                })
                .chain(exits.iter().map(|exit| format!("exit:{}", exit.name)))
                .collect(),
            Vec::new(),
        ),
        HirSymbolBody::None => (Vec::new(), Vec::new(), Vec::new(), Vec::new()),
    };
    AdapterSymbolSnapshot {
        kind: symbol.kind.as_str().to_string(),
        name: symbol.name.clone(),
        exported: symbol.exported,
        is_async: symbol.is_async,
        signature: render_symbol_signature(symbol),
        type_params: symbol.type_params.clone(),
        params: symbol
            .params
            .iter()
            .map(build_adapter_param_snapshot)
            .collect(),
        return_type: symbol.return_type.as_ref().map(ToString::to_string),
        forewords: symbol
            .forewords
            .iter()
            .map(render_foreword_app_text)
            .collect(),
        fields,
        methods,
        variants,
        assoc_types,
        body_fingerprint: render_symbol_fingerprint(symbol),
    }
}

fn build_adapter_directive_snapshot(
    directive: &arcana_hir::HirDirective,
) -> AdapterDirectiveSnapshot {
    AdapterDirectiveSnapshot {
        kind: directive.kind.as_str().to_string(),
        path: directive.path.join("."),
        alias: directive.alias.clone(),
        forewords: directive
            .forewords
            .iter()
            .map(render_foreword_app_text)
            .collect(),
    }
}

fn visible_adapter_catalog(
    package: &HirWorkspacePackage,
    registry: &ForewordRegistry,
) -> Vec<AdapterCatalogEntry> {
    let mut entries = registry
        .exports
        .values()
        .filter(|export| {
            export.is_builtin()
                || export.exposed_package_id == package.package_id
                || (export.public
                    && package
                        .direct_dep_ids
                        .values()
                        .any(|dep_id| dep_id == &export.exposed_package_id))
        })
        .map(|export| AdapterCatalogEntry {
            exposed_name: export.exposed_name.join("."),
            qualified_name: export.definition.qualified_name.join("."),
            tier: export.catalog_tier().to_string(),
            action: export.definition.action.as_str().to_string(),
            retention: export.definition.retention.as_str().to_string(),
            targets: builtin_foreword_targets(&export.definition.targets),
            provider_package_id: export.provider_package_id.clone(),
        })
        .collect::<Vec<_>>();
    for entry in registry
        .catalog
        .iter()
        .filter(|entry| entry.tier == "builtin")
    {
        if entries.iter().any(|candidate| {
            candidate.exposed_name == entry.exposed_name
                && candidate.provider_package_id == entry.provider_package_id
        }) {
            continue;
        }
        entries.push(AdapterCatalogEntry {
            exposed_name: entry.exposed_name.clone(),
            qualified_name: entry.qualified_name.clone(),
            tier: entry.tier.clone(),
            action: entry.action.clone(),
            retention: entry.retention.clone(),
            targets: entry.targets.clone(),
            provider_package_id: entry.provider_package_id.clone(),
        });
    }
    entries.sort_by(|left, right| {
        left.exposed_name
            .cmp(&right.exposed_name)
            .then_with(|| left.provider_package_id.cmp(&right.provider_package_id))
    });
    entries
}

fn target_path_for_symbol(
    module_id: &str,
    symbol_name: &str,
    container_name: Option<&str>,
) -> String {
    match container_name {
        Some(container_name) => format!("{module_id}.{container_name}.{symbol_name}"),
        None => format!("{module_id}.{symbol_name}"),
    }
}

fn target_path_for_param(
    module_id: &str,
    symbol_name: &str,
    param_name: &str,
    container_name: Option<&str>,
) -> String {
    format!(
        "{}({param_name})",
        target_path_for_symbol(module_id, symbol_name, container_name)
    )
}

fn target_path_for_field(
    module_id: &str,
    symbol_name: &str,
    field_name: &str,
    container_name: Option<&str>,
) -> String {
    format!(
        "{}.{field_name}",
        target_path_for_symbol(module_id, symbol_name, container_name)
    )
}

fn impl_container_name(impl_decl: &HirImplDecl) -> String {
    let target = impl_decl.target_type.to_string();
    match &impl_decl.trait_path {
        Some(trait_path) => format!("{target}:{}", arcana_hir::render_hir_trait_ref(trait_path)),
        None => target,
    }
}

fn impl_target_path(module_id: &str, impl_decl: &HirImplDecl) -> String {
    format!("{module_id}::impl({})", impl_container_name(impl_decl))
}

fn resolve_foreword_adapter_product<'a>(
    workspace: &'a HirWorkspaceSummary,
    export: &ResolvedForewordExport,
    handler: &arcana_hir::HirForewordHandler,
) -> Result<
    (
        &'a HirWorkspacePackage,
        &'a arcana_hir::HirForewordAdapterProduct,
    ),
    String,
> {
    let provider_package = workspace
        .package_by_id(&export.provider_package_id)
        .ok_or_else(|| {
            format!(
                "executable foreword provider package `{}` is not loaded",
                export.provider_package_id
            )
        })?;
    let product = provider_package
        .foreword_products
        .get(&handler.product)
        .ok_or_else(|| {
            format!(
                "foreword handler `{}` references unknown product `{}`",
                handler.qualified_name.join("."),
                handler.product
            )
        })?;
    Ok((provider_package, product))
}

fn execute_foreword_adapter(
    provider_package: &HirWorkspacePackage,
    product: &arcana_hir::HirForewordAdapterProduct,
    request: &ForewordAdapterRequest,
) -> Result<ForewordAdapterResponse, String> {
    let materialized = materialize_foreword_adapter_artifact(provider_package, product)?;
    let (program, args) = if let Some(runner) = &materialized.runner_program {
        let mut args = Vec::with_capacity(materialized.args.len() + 1);
        args.extend(materialized.args.iter().cloned());
        args.push(materialized.product_arg_path.clone());
        (runner.clone(), args)
    } else {
        (
            materialized.product_program.clone(),
            materialized.args.clone(),
        )
    };
    let request_json = serde_json::to_vec(request)
        .map_err(|err| format!("failed to encode foreword adapter request: {err}"))?;
    let mut child = Command::new(&program)
        .args(&args)
        .current_dir(external_process_path(&provider_package.root_dir))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| {
            format!(
                "failed to launch foreword adapter `{}` from `{}`: {err}",
                product.name,
                provider_package.root_dir.display()
            )
        })?;
    if let Some(stdin) = child.stdin.as_mut() {
        use std::io::Write;
        stdin.write_all(&request_json).map_err(|err| {
            format!(
                "failed to write foreword adapter request to `{}`: {err}",
                product.name
            )
        })?;
    }
    let output = child.wait_with_output().map_err(|err| {
        format!(
            "failed to wait for foreword adapter `{}`: {err}",
            product.name
        )
    })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            format!("status {}", output.status)
        };
        return Err(format!(
            "foreword adapter `{}` failed: {}",
            product.name, detail
        ));
    }
    let response: ForewordAdapterResponse =
        serde_json::from_slice(&output.stdout).map_err(|err| {
            format!(
                "foreword adapter `{}` returned invalid JSON: {err}",
                product.name
            )
        })?;
    if response.version != FOREWORD_ADAPTER_PROTOCOL_VERSION {
        return Err(format!(
            "foreword adapter `{}` returned protocol `{}` instead of `{}`",
            product.name, response.version, FOREWORD_ADAPTER_PROTOCOL_VERSION
        ));
    }
    Ok(response)
}

fn parse_adapter_symbol_snippet(module_id: &str, snippet: &str) -> Result<HirSymbol, String> {
    let lowered = lower_module_text(module_id.to_string(), snippet)?;
    if !lowered.directives.is_empty()
        || !lowered.lang_items.is_empty()
        || !lowered.memory_specs.is_empty()
        || !lowered.foreword_definitions.is_empty()
        || !lowered.foreword_handlers.is_empty()
        || !lowered.foreword_aliases.is_empty()
        || !lowered.impls.is_empty()
        || lowered.symbols.len() != 1
    {
        return Err(
            "adapter symbol snippet must produce exactly one symbol declaration".to_string(),
        );
    }
    Ok(lowered.symbols.into_iter().next().expect("single symbol"))
}

fn parse_adapter_directive_snippet(
    module_id: &str,
    snippet: &str,
) -> Result<arcana_hir::HirDirective, String> {
    let lowered = lower_module_text(module_id.to_string(), snippet)?;
    if !lowered.lang_items.is_empty()
        || !lowered.memory_specs.is_empty()
        || !lowered.foreword_definitions.is_empty()
        || !lowered.foreword_handlers.is_empty()
        || !lowered.foreword_aliases.is_empty()
        || !lowered.symbols.is_empty()
        || !lowered.impls.is_empty()
        || lowered.directives.len() != 1
    {
        return Err(
            "adapter directive snippet must produce exactly one import/use/reexport declaration"
                .to_string(),
        );
    }
    Ok(lowered
        .directives
        .into_iter()
        .next()
        .expect("single directive"))
}

fn apply_adapter_diagnostics(
    response: &ForewordAdapterResponse,
    module_path: &Path,
    span: Span,
    qualified_name: &str,
    diagnostic_namespace: Option<&str>,
    policy: &LintPolicy,
    warnings: &mut Vec<CheckWarning>,
    errors: &mut Vec<Diagnostic>,
) {
    for diagnostic in &response.diagnostics {
        let rendered = format!(
            "foreword adapter `{qualified_name}`: {}",
            diagnostic.message
        );
        if diagnostic.severity == "warning" {
            let lint_name = diagnostic
                .lint
                .as_deref()
                .map(|lint| {
                    if lint.contains('.') {
                        lint.to_string()
                    } else if let Some(namespace) = diagnostic_namespace {
                        format!("{namespace}.{lint}")
                    } else {
                        format!("{qualified_name}.{lint}")
                    }
                })
                .unwrap_or_else(|| {
                    diagnostic_namespace
                        .map(|namespace| format!("{namespace}.adapter"))
                        .unwrap_or_else(|| format!("{qualified_name}.adapter"))
                });
            if lint_is_allowed(policy, &lint_name) {
                continue;
            }
            if lint_is_denied(policy, &lint_name) {
                errors.push(Diagnostic {
                    path: module_path.to_path_buf(),
                    line: span.line,
                    column: span.column,
                    message: rendered,
                });
            } else {
                warnings.push(CheckWarning {
                    path: module_path.to_path_buf(),
                    line: span.line,
                    column: span.column,
                    message: rendered,
                });
            }
        } else {
            errors.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: span.line,
                column: span.column,
                message: rendered,
            });
        }
    }
}

fn validate_user_foreword_payload(
    app: &arcana_hir::HirForewordApp,
    definition: &arcana_hir::HirForewordDefinition,
) -> Result<(), String> {
    if definition.payload.is_empty() && app.args.is_empty() {
        return Ok(());
    }
    if definition.payload.is_empty() {
        return Err(format!(
            "`#{}` does not accept a payload",
            app.path.join(".")
        ));
    }
    let mut assigned = BTreeMap::<String, &arcana_hir::HirForewordArg>::new();
    let mut positional_index = 0usize;
    for arg in &app.args {
        if let Some(name) = &arg.name {
            let Some(field) = definition.payload.iter().find(|field| field.name == *name) else {
                return Err(format!(
                    "`#{}` has no payload field `{name}`",
                    app.path.join(".")
                ));
            };
            if assigned.insert(field.name.clone(), arg).is_some() {
                return Err(format!(
                    "`#{}` payload field `{name}` is assigned more than once",
                    app.path.join(".")
                ));
            }
        } else {
            let Some(field) = definition.payload.get(positional_index) else {
                return Err(format!(
                    "`#{}` received too many positional payload values",
                    app.path.join(".")
                ));
            };
            if assigned.insert(field.name.clone(), arg).is_some() {
                return Err(format!(
                    "`#{}` payload field `{}` is assigned more than once",
                    app.path.join("."),
                    field.name
                ));
            }
            positional_index += 1;
        }
    }
    for field in &definition.payload {
        let Some(arg) = assigned.get(&field.name).copied() else {
            if field.optional {
                continue;
            }
            return Err(format!(
                "`#{}` is missing required payload field `{}`",
                app.path.join("."),
                field.name
            ));
        };
        let valid = match field.ty {
            arcana_hir::HirForewordPayloadType::Bool => {
                matches!(arg.typed_value, arcana_hir::HirForewordArgValue::Bool(_))
            }
            arcana_hir::HirForewordPayloadType::Int => {
                matches!(arg.typed_value, arcana_hir::HirForewordArgValue::Int(_))
            }
            arcana_hir::HirForewordPayloadType::Str => {
                matches!(arg.typed_value, arcana_hir::HirForewordArgValue::Str(_))
            }
            arcana_hir::HirForewordPayloadType::Symbol => {
                matches!(arg.typed_value, arcana_hir::HirForewordArgValue::Symbol(_))
            }
            arcana_hir::HirForewordPayloadType::Path => matches!(
                arg.typed_value,
                arcana_hir::HirForewordArgValue::Path(_)
                    | arcana_hir::HirForewordArgValue::Symbol(_)
            ),
        };
        if !valid {
            return Err(format!(
                "`#{}` payload field `{}` must be {}",
                app.path.join("."),
                field.name,
                field.ty.as_str()
            ));
        }
    }
    Ok(())
}

fn target_allowed_for_definition(
    definition: &arcana_hir::HirForewordDefinition,
    target_kind: &str,
) -> bool {
    definition
        .targets
        .iter()
        .any(|target| target.as_str() == target_kind)
}

fn dependency_allows_executable_forewords(
    package: &HirWorkspacePackage,
    exposed_package_id: &str,
) -> bool {
    if exposed_package_id == package.package_id {
        return true;
    }
    package
        .direct_dep_ids
        .iter()
        .filter_map(|(alias, dep_id)| (dep_id == exposed_package_id).then_some(alias))
        .any(|alias| package.executable_foreword_deps.contains(alias))
}

fn execute_executable_foreword_app(
    workspace: &HirWorkspaceSummary,
    package: &HirWorkspacePackage,
    module_path: &Path,
    module_id: &str,
    registry: &ForewordRegistry,
    adapter_cache: &mut BTreeMap<String, ForewordAdapterResponse>,
    app: &arcana_hir::HirForewordApp,
    export: &ResolvedForewordExport,
    target_kind: &str,
    target_path: &str,
    target: AdapterTargetSnapshot,
    policy: &LintPolicy,
    warnings: &mut Vec<CheckWarning>,
    errors: &mut Vec<Diagnostic>,
) -> Option<ExecutedTransform> {
    if !dependency_allows_executable_forewords(package, &export.exposed_package_id) {
        errors.push(Diagnostic {
            path: module_path.to_path_buf(),
            line: app.span.line,
            column: app.span.column,
            message: format!(
                "executable foreword `#{}` requires `executable_forewords = true` on the dependency that exports it",
                app.path.join(".")
            ),
        });
        return None;
    }
    let Some(handler) = &export.handler else {
        errors.push(Diagnostic {
            path: module_path.to_path_buf(),
            line: app.span.line,
            column: app.span.column,
            message: format!(
                "executable foreword `#{}` is missing a handler binding",
                app.path.join(".")
            ),
        });
        return None;
    };
    if handler.protocol != "stdio-v1" {
        errors.push(Diagnostic {
            path: module_path.to_path_buf(),
            line: app.span.line,
            column: app.span.column,
            message: format!(
                "foreword handler `{}` uses unsupported protocol `{}`",
                handler.qualified_name.join("."),
                handler.protocol
            ),
        });
        return None;
    }
    let rendered_args = render_foreword_args(app);
    let dependency_opt_in_enabled =
        dependency_allows_executable_forewords(package, &export.exposed_package_id);
    let (provider_package, product) =
        match resolve_foreword_adapter_product(workspace, export, handler) {
            Ok(value) => value,
            Err(message) => {
                errors.push(Diagnostic {
                    path: module_path.to_path_buf(),
                    line: app.span.line,
                    column: app.span.column,
                    message,
                });
                return None;
            }
        };
    let visible_forewords = visible_adapter_catalog(package, registry);
    let artifact = build_adapter_artifact_identity(provider_package, product);
    let cache_key = build_foreword_adapter_cache_key(
        package,
        export,
        handler,
        product,
        &target,
        &rendered_args,
        &visible_forewords,
        dependency_opt_in_enabled,
        &artifact,
    );
    let request = ForewordAdapterRequest {
        version: FOREWORD_ADAPTER_PROTOCOL_VERSION.to_string(),
        protocol: handler.protocol.clone(),
        cache_key: cache_key.clone(),
        toolchain_version: env!("CARGO_PKG_VERSION").to_string(),
        dependency_opt_in_enabled,
        package: AdapterPackageSnapshot {
            package_id: package.package_id.clone(),
            package_name: package.summary.package_name.clone(),
            root_dir: external_process_path_string(&package.root_dir),
            module_id: module_id.to_string(),
        },
        foreword: AdapterForewordSnapshot {
            applied_name: app.path.join("."),
            resolved_name: export.exposed_name.join("."),
            tier: export.definition.tier.as_str().to_string(),
            visibility: export.definition.visibility.as_str().to_string(),
            phase: export.definition.phase.as_str().to_string(),
            action: export.definition.action.as_str().to_string(),
            retention: export.definition.retention.as_str().to_string(),
            targets: export
                .definition
                .targets
                .iter()
                .map(|target| target.as_str().to_string())
                .collect(),
            diagnostic_namespace: export.definition.diagnostic_namespace.clone(),
            payload_schema: build_adapter_payload_schema(&export.definition),
            repeatable: export.definition.repeatable,
            conflicts: export
                .definition
                .conflicts
                .iter()
                .map(|conflict| conflict.join("."))
                .collect(),
            args: lower_adapter_payload_args(&app.args),
            provider_package_id: export.provider_package_id.clone(),
            exposed_package_id: export.exposed_package_id.clone(),
            handler: Some(handler.qualified_name.join(".")),
            entry: Some(handler.entry.clone()),
        },
        target,
        visible_forewords,
        artifact,
    };
    let response = if let Some(cached) = adapter_cache.get(&cache_key).cloned() {
        cached
    } else {
        let response = match execute_foreword_adapter(provider_package, product, &request) {
            Ok(response) => response,
            Err(message) => {
                errors.push(Diagnostic {
                    path: module_path.to_path_buf(),
                    line: app.span.line,
                    column: app.span.column,
                    message,
                });
                return None;
            }
        };
        adapter_cache.insert(cache_key, response.clone());
        response
    };
    apply_adapter_diagnostics(
        &response,
        module_path,
        app.span,
        &export.exposed_name.join("."),
        export.definition.diagnostic_namespace.as_deref(),
        policy,
        warnings,
        errors,
    );
    Some(ExecutedTransform {
        response,
        generated_by: arcana_hir::HirGeneratedByForeword {
            applied_name: app.path.join("."),
            resolved_name: export.exposed_name.join("."),
            provider_package_id: export.provider_package_id.clone(),
            owner_kind: target_kind.to_string(),
            owner_path: target_path.to_string(),
            retention: export.definition.retention,
            args: app.args.clone(),
        },
    })
}

fn maybe_execute_transform_app(
    workspace: &HirWorkspaceSummary,
    package: &HirWorkspacePackage,
    module_path: &Path,
    module_id: &str,
    registry: &ForewordRegistry,
    executed: &mut BTreeSet<ExecutedTransformKey>,
    adapter_cache: &mut BTreeMap<String, ForewordAdapterResponse>,
    app: &arcana_hir::HirForewordApp,
    target_kind: &str,
    target_path: &str,
    target: AdapterTargetSnapshot,
    policy: &LintPolicy,
    warnings: &mut Vec<CheckWarning>,
    errors: &mut Vec<Diagnostic>,
) -> Option<ExecutedTransform> {
    let export = resolve_user_foreword_export(package, app, registry)?;
    if export.definition.tier != arcana_hir::HirForewordTier::Executable
        || export.definition.action != arcana_hir::HirForewordAction::Transform
    {
        return None;
    }
    let key = ExecutedTransformKey {
        package_id: package.package_id.clone(),
        module_id: module_id.to_string(),
        target_kind: target_kind.to_string(),
        target_path: target_path.to_string(),
        line: app.span.line,
        column: app.span.column,
        qualified_name: export.exposed_name.join("."),
        args: render_foreword_args(app),
    };
    if !executed.insert(key) {
        return None;
    }
    execute_executable_foreword_app(
        workspace,
        package,
        module_path,
        module_id,
        registry,
        adapter_cache,
        app,
        export,
        target_kind,
        target_path,
        target,
        policy,
        warnings,
        errors,
    )
}

fn maybe_execute_metadata_app(
    workspace: &HirWorkspaceSummary,
    package: &HirWorkspacePackage,
    module_path: &Path,
    module_id: &str,
    registry: &ForewordRegistry,
    adapter_cache: &mut BTreeMap<String, ForewordAdapterResponse>,
    app: &arcana_hir::HirForewordApp,
    target_kind: &str,
    target_path: &str,
    target: AdapterTargetSnapshot,
    policy: &LintPolicy,
    warnings: &mut Vec<CheckWarning>,
    errors: &mut Vec<Diagnostic>,
) -> Option<ExecutedTransform> {
    let export = resolve_user_foreword_export(package, app, registry)?;
    if export.definition.tier != arcana_hir::HirForewordTier::Executable
        || export.definition.action != arcana_hir::HirForewordAction::Metadata
    {
        return None;
    }
    execute_executable_foreword_app(
        workspace,
        package,
        module_path,
        module_id,
        registry,
        adapter_cache,
        app,
        export,
        target_kind,
        target_path,
        target,
        policy,
        warnings,
        errors,
    )
}

fn build_generated_symbol_name_key(
    generated_by: &arcana_hir::HirGeneratedByForeword,
    symbol: &HirSymbol,
) -> String {
    hash_json_hex(&(
        &generated_by.applied_name,
        &generated_by.resolved_name,
        &generated_by.provider_package_id,
        &generated_by.owner_kind,
        &generated_by.owner_path,
        render_hir_foreword_args(&generated_by.args),
        symbol.kind.as_str(),
        &symbol.name,
        render_symbol_signature(symbol),
    ))
}

fn parse_appended_symbols(
    module_id: &str,
    snippets: &[String],
    generated_by: &arcana_hir::HirGeneratedByForeword,
) -> Result<Vec<HirSymbol>, String> {
    let mut items = snippets
        .iter()
        .map(|snippet| {
            let mut symbol = parse_adapter_symbol_snippet(module_id, snippet)?;
            symbol.generated_by = Some(generated_by.clone());
            symbol.generated_name_key =
                Some(build_generated_symbol_name_key(generated_by, &symbol));
            Ok(symbol)
        })
        .collect::<Result<Vec<_>, String>>()?;
    items.sort_by(|left, right| {
        left.generated_name_key
            .cmp(&right.generated_name_key)
            .then_with(|| left.name.cmp(&right.name))
    });
    Ok(items)
}

fn apply_symbol_transform_response(
    module_id: &str,
    module_path: &Path,
    app: &arcana_hir::HirForewordApp,
    owner_label: &str,
    target_kind: &str,
    target_path: &str,
    owner_public: bool,
    symbol: &mut HirSymbol,
    executed: ExecutedTransform,
    appended_symbols: &mut Vec<HirSymbol>,
    _appended_impls: &mut Vec<HirImplDecl>,
    emitted_metadata: &mut Vec<arcana_hir::HirEmittedForewordMetadata>,
    registration_rows: &mut Vec<arcana_hir::HirForewordRegistrationRow>,
    changed: &mut bool,
    errors: &mut Vec<Diagnostic>,
) {
    let ExecutedTransform {
        response,
        generated_by,
    } = executed;
    let allow_adjacent_symbol_siblings = matches!(
        target_kind,
        "fn" | "record"
            | "obj"
            | "owner"
            | "enum"
            | "opaque_type"
            | "trait"
            | "behavior"
            | "system"
            | "const"
    );
    if let Some(replacement) = response.replace_owner {
        match parse_adapter_symbol_snippet(module_id, &replacement) {
            Ok(mut parsed) if parsed.name == symbol.name && parsed.kind == symbol.kind => {
                parsed.generated_by = symbol.generated_by.clone();
                parsed.generated_name_key = symbol.generated_name_key.clone();
                *symbol = parsed;
                *changed = true;
            }
            Ok(parsed) => errors.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: app.span.line,
                column: app.span.column,
                message: format!(
                    "{owner_label} foreword adapter replacement for `{}` must keep owner `{}` with kind `{}` (got `{}` `{}`)",
                    target_path,
                    symbol.name,
                    symbol.kind.as_str(),
                    parsed.kind.as_str(),
                    parsed.name
                ),
            }),
            Err(message) => errors.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: app.span.line,
                column: app.span.column,
                message,
            }),
        }
    }
    if response.replace_directive.is_some() {
        errors.push(Diagnostic {
            path: module_path.to_path_buf(),
            line: app.span.line,
            column: app.span.column,
            message: "symbol foreword adapters cannot replace directives".to_string(),
        });
    }
    if !response.append_symbols.is_empty() && !allow_adjacent_symbol_siblings {
        errors.push(Diagnostic {
            path: module_path.to_path_buf(),
            line: app.span.line,
            column: app.span.column,
            message: format!(
                "{owner_label} foreword adapters may only append sibling declarations for top-level declaration targets"
            ),
        });
    } else {
        match parse_appended_symbols(module_id, &response.append_symbols, &generated_by) {
            Ok(mut items) => {
                if !items.is_empty() {
                    *changed = true;
                }
                appended_symbols.append(&mut items);
            }
            Err(message) => errors.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: app.span.line,
                column: app.span.column,
                message,
            }),
        }
    }
    if !response.append_impls.is_empty() {
        errors.push(Diagnostic {
            path: module_path.to_path_buf(),
            line: app.span.line,
            column: app.span.column,
            message:
                "symbol foreword adapters cannot append impl blocks because sibling emission is limited to adjacent top-level declarations"
                    .to_string(),
        });
    }
    emitted_metadata.extend(collect_emitted_metadata(
        module_path,
        app,
        owner_public,
        &generated_by,
        &response.emitted_metadata,
        errors,
    ));
    registration_rows.extend(collect_registration_rows(
        module_path,
        app,
        owner_public,
        &generated_by,
        &response.registration_rows,
        errors,
    ));
}

fn apply_directive_transform_response(
    module_id: &str,
    module_path: &Path,
    app: &arcana_hir::HirForewordApp,
    target_path: &str,
    owner_public: bool,
    directive: &mut arcana_hir::HirDirective,
    executed: ExecutedTransform,
    _appended_symbols: &mut Vec<HirSymbol>,
    _appended_impls: &mut Vec<HirImplDecl>,
    emitted_metadata: &mut Vec<arcana_hir::HirEmittedForewordMetadata>,
    registration_rows: &mut Vec<arcana_hir::HirForewordRegistrationRow>,
    changed: &mut bool,
    errors: &mut Vec<Diagnostic>,
) {
    let ExecutedTransform {
        response,
        generated_by,
    } = executed;
    if let Some(replacement) = response.replace_directive {
        match parse_adapter_directive_snippet(module_id, &replacement) {
            Ok(parsed) if parsed.kind == directive.kind => {
                *directive = parsed;
                *changed = true;
            }
            Ok(parsed) => errors.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: app.span.line,
                column: app.span.column,
                message: format!(
                    "directive foreword adapter replacement for `{}` must keep directive kind `{}` (got `{}`)",
                    target_path,
                    directive.kind.as_str(),
                    parsed.kind.as_str()
                ),
            }),
            Err(message) => errors.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: app.span.line,
                column: app.span.column,
                message,
            }),
        }
    }
    if response.replace_owner.is_some() {
        errors.push(Diagnostic {
            path: module_path.to_path_buf(),
            line: app.span.line,
            column: app.span.column,
            message: "directive foreword adapters cannot replace symbols".to_string(),
        });
    }
    if !response.append_symbols.is_empty() {
        errors.push(Diagnostic {
            path: module_path.to_path_buf(),
            line: app.span.line,
            column: app.span.column,
            message:
                "directive foreword adapters cannot append sibling declarations because import/use/reexport ordering is preserved separately from declaration lists"
                    .to_string(),
        });
    }
    if !response.append_impls.is_empty() {
        errors.push(Diagnostic {
            path: module_path.to_path_buf(),
            line: app.span.line,
            column: app.span.column,
            message:
                "directive foreword adapters cannot append impl blocks because sibling emission is limited to adjacent declaration slots"
                    .to_string(),
        });
    }
    emitted_metadata.extend(collect_emitted_metadata(
        module_path,
        app,
        owner_public,
        &generated_by,
        &response.emitted_metadata,
        errors,
    ));
    registration_rows.extend(collect_registration_rows(
        module_path,
        app,
        owner_public,
        &generated_by,
        &response.registration_rows,
        errors,
    ));
}

fn apply_metadata_response(
    module_path: &Path,
    app: &arcana_hir::HirForewordApp,
    owner_public: bool,
    executed: ExecutedTransform,
    emitted_metadata: &mut Vec<arcana_hir::HirEmittedForewordMetadata>,
    registration_rows: &mut Vec<arcana_hir::HirForewordRegistrationRow>,
    errors: &mut Vec<Diagnostic>,
) {
    let ExecutedTransform {
        response,
        generated_by,
    } = executed;
    if response.replace_owner.is_some() {
        errors.push(Diagnostic {
            path: module_path.to_path_buf(),
            line: app.span.line,
            column: app.span.column,
            message: "metadata foreword adapters cannot replace symbols".to_string(),
        });
    }
    if response.replace_directive.is_some() {
        errors.push(Diagnostic {
            path: module_path.to_path_buf(),
            line: app.span.line,
            column: app.span.column,
            message: "metadata foreword adapters cannot replace directives".to_string(),
        });
    }
    if !response.append_symbols.is_empty() {
        errors.push(Diagnostic {
            path: module_path.to_path_buf(),
            line: app.span.line,
            column: app.span.column,
            message: "metadata foreword adapters cannot append symbols".to_string(),
        });
    }
    if !response.append_impls.is_empty() {
        errors.push(Diagnostic {
            path: module_path.to_path_buf(),
            line: app.span.line,
            column: app.span.column,
            message: "metadata foreword adapters cannot append impl blocks".to_string(),
        });
    }
    emitted_metadata.extend(collect_emitted_metadata(
        module_path,
        app,
        owner_public,
        &generated_by,
        &response.emitted_metadata,
        errors,
    ));
    registration_rows.extend(collect_registration_rows(
        module_path,
        app,
        owner_public,
        &generated_by,
        &response.registration_rows,
        errors,
    ));
}

fn transform_symbol(
    workspace: &HirWorkspaceSummary,
    package: &HirWorkspacePackage,
    module_path: &Path,
    module_id: &str,
    registry: &ForewordRegistry,
    executed: &mut BTreeSet<ExecutedTransformKey>,
    adapter_cache: &mut BTreeMap<String, ForewordAdapterResponse>,
    mut symbol: HirSymbol,
    target_kind: &str,
    container_kind: Option<&str>,
    container_name: Option<&str>,
    public: bool,
    inherited_lint_layers: &[LintPolicyLayer],
    emitted_metadata: &mut Vec<arcana_hir::HirEmittedForewordMetadata>,
    registration_rows: &mut Vec<arcana_hir::HirForewordRegistrationRow>,
    warnings: &mut Vec<CheckWarning>,
    errors: &mut Vec<Diagnostic>,
) -> (HirSymbol, Vec<HirSymbol>, Vec<HirImplDecl>, bool) {
    let mut changed = false;
    let mut appended_symbols = Vec::new();
    let mut appended_impls = Vec::new();
    let target_path = target_path_for_symbol(module_id, &symbol.name, container_name);
    let symbol_lint_layer = lint_policy_layer_from_forewords(&symbol.forewords);

    for app in symbol.forewords.clone() {
        let target = AdapterTargetSnapshot {
            kind: target_kind.to_string(),
            path: target_path.clone(),
            public,
            owner_kind: "symbol".to_string(),
            owner_symbol: Some(build_adapter_symbol_snapshot(&symbol)),
            owner_directive: None,
            selected_field: None,
            selected_param: None,
            selected_method_name: None,
            container_kind: container_kind.map(ToString::to_string),
            container_name: container_name.map(ToString::to_string),
        };
        if let Some(response) = maybe_execute_transform_app(
            workspace,
            package,
            module_path,
            module_id,
            registry,
            executed,
            adapter_cache,
            &app,
            target_kind,
            &target_path,
            target,
            &lint_policy_with_inherited([symbol_lint_layer.clone()], inherited_lint_layers),
            warnings,
            errors,
        ) {
            apply_symbol_transform_response(
                module_id,
                module_path,
                &app,
                "symbol",
                target_kind,
                &target_path,
                public,
                &mut symbol,
                response,
                &mut appended_symbols,
                &mut appended_impls,
                emitted_metadata,
                registration_rows,
                &mut changed,
                errors,
            );
        }
    }

    let param_names = symbol
        .params
        .iter()
        .map(|param| param.name.clone())
        .collect::<Vec<_>>();
    for param_name in param_names {
        let Some(current_param) = symbol
            .params
            .iter()
            .find(|param| param.name == param_name)
            .cloned()
        else {
            continue;
        };
        let param_target_path =
            target_path_for_param(module_id, &symbol.name, &param_name, container_name);
        for app in current_param.forewords {
            let target = AdapterTargetSnapshot {
                kind: "param".to_string(),
                path: param_target_path.clone(),
                public,
                owner_kind: "symbol".to_string(),
                owner_symbol: Some(build_adapter_symbol_snapshot(&symbol)),
                owner_directive: None,
                selected_field: None,
                selected_param: symbol
                    .params
                    .iter()
                    .find(|param| param.name == param_name)
                    .map(build_adapter_param_snapshot),
                selected_method_name: None,
                container_kind: container_kind.map(ToString::to_string),
                container_name: container_name.map(ToString::to_string),
            };
            if let Some(response) = maybe_execute_transform_app(
                workspace,
                package,
                module_path,
                module_id,
                registry,
                executed,
                adapter_cache,
                &app,
                "param",
                &param_target_path,
                target,
                &lint_policy_with_inherited(
                    [
                        lint_policy_layer_from_forewords(
                            &symbol
                                .params
                                .iter()
                                .find(|param| param.name == param_name)
                                .map(|param| param.forewords.clone())
                                .unwrap_or_default(),
                        ),
                        symbol_lint_layer.clone(),
                    ],
                    inherited_lint_layers,
                ),
                warnings,
                errors,
            ) {
                apply_symbol_transform_response(
                    module_id,
                    module_path,
                    &app,
                    "param",
                    "param",
                    &param_target_path,
                    public,
                    &mut symbol,
                    response,
                    &mut appended_symbols,
                    &mut appended_impls,
                    emitted_metadata,
                    registration_rows,
                    &mut changed,
                    errors,
                );
            }
        }
    }

    let field_names = match &symbol.body {
        HirSymbolBody::Record { fields } | HirSymbolBody::Object { fields, .. } => fields
            .iter()
            .map(|field| field.name.clone())
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };
    for field_name in field_names {
        let Some(current_field) = (match &symbol.body {
            HirSymbolBody::Record { fields } | HirSymbolBody::Object { fields, .. } => fields
                .iter()
                .find(|field| field.name == field_name)
                .cloned(),
            _ => None,
        }) else {
            continue;
        };
        let field_target_path =
            target_path_for_field(module_id, &symbol.name, &field_name, container_name);
        for app in current_field.forewords {
            let target = AdapterTargetSnapshot {
                kind: "field".to_string(),
                path: field_target_path.clone(),
                public,
                owner_kind: "symbol".to_string(),
                owner_symbol: Some(build_adapter_symbol_snapshot(&symbol)),
                owner_directive: None,
                selected_field: match &symbol.body {
                    HirSymbolBody::Record { fields } | HirSymbolBody::Object { fields, .. } => {
                        fields
                            .iter()
                            .find(|field| field.name == field_name)
                            .map(build_adapter_field_snapshot)
                    }
                    _ => None,
                },
                selected_param: None,
                selected_method_name: None,
                container_kind: container_kind.map(ToString::to_string),
                container_name: container_name.map(ToString::to_string),
            };
            if let Some(response) = maybe_execute_transform_app(
                workspace,
                package,
                module_path,
                module_id,
                registry,
                executed,
                adapter_cache,
                &app,
                "field",
                &field_target_path,
                target,
                &lint_policy_with_inherited(
                    [
                        lint_policy_layer_from_forewords(
                            &(match &symbol.body {
                                HirSymbolBody::Record { fields }
                                | HirSymbolBody::Object { fields, .. } => fields
                                    .iter()
                                    .find(|field| field.name == field_name)
                                    .map(|field| field.forewords.clone())
                                    .unwrap_or_default(),
                                _ => Vec::new(),
                            }),
                        ),
                        symbol_lint_layer.clone(),
                    ],
                    inherited_lint_layers,
                ),
                warnings,
                errors,
            ) {
                apply_symbol_transform_response(
                    module_id,
                    module_path,
                    &app,
                    "field",
                    "field",
                    &field_target_path,
                    public,
                    &mut symbol,
                    response,
                    &mut appended_symbols,
                    &mut appended_impls,
                    emitted_metadata,
                    registration_rows,
                    &mut changed,
                    errors,
                );
            }
        }
    }

    match &mut symbol.body {
        HirSymbolBody::Object { methods, .. } => {
            let old_methods = std::mem::take(methods);
            let mut next_methods = Vec::with_capacity(old_methods.len());
            let child_inherited_layers = std::iter::once(symbol_lint_layer.clone())
                .chain(inherited_lint_layers.iter().cloned())
                .collect::<Vec<_>>();
            for method in old_methods {
                let (method, mut extra_symbols, mut extra_impls, method_changed) = transform_symbol(
                    workspace,
                    package,
                    module_path,
                    module_id,
                    registry,
                    executed,
                    adapter_cache,
                    method,
                    "impl_method",
                    Some("object"),
                    Some(&symbol.name),
                    public,
                    &child_inherited_layers,
                    emitted_metadata,
                    registration_rows,
                    warnings,
                    errors,
                );
                changed |= method_changed;
                next_methods.push(method);
                appended_symbols.append(&mut extra_symbols);
                appended_impls.append(&mut extra_impls);
            }
            *methods = next_methods;
        }
        HirSymbolBody::Trait { methods, .. } => {
            let old_methods = std::mem::take(methods);
            let mut next_methods = Vec::with_capacity(old_methods.len());
            let child_inherited_layers = std::iter::once(symbol_lint_layer.clone())
                .chain(inherited_lint_layers.iter().cloned())
                .collect::<Vec<_>>();
            for method in old_methods {
                let (method, mut extra_symbols, mut extra_impls, method_changed) = transform_symbol(
                    workspace,
                    package,
                    module_path,
                    module_id,
                    registry,
                    executed,
                    adapter_cache,
                    method,
                    "trait_method",
                    Some("trait"),
                    Some(&symbol.name),
                    public,
                    &child_inherited_layers,
                    emitted_metadata,
                    registration_rows,
                    warnings,
                    errors,
                );
                changed |= method_changed;
                next_methods.push(method);
                appended_symbols.append(&mut extra_symbols);
                appended_impls.append(&mut extra_impls);
            }
            *methods = next_methods;
        }
        _ => {}
    }

    (symbol, appended_symbols, appended_impls, changed)
}

fn transform_impl(
    workspace: &HirWorkspaceSummary,
    package: &HirWorkspacePackage,
    module_path: &Path,
    module_id: &str,
    registry: &ForewordRegistry,
    executed: &mut BTreeSet<ExecutedTransformKey>,
    adapter_cache: &mut BTreeMap<String, ForewordAdapterResponse>,
    mut impl_decl: HirImplDecl,
    emitted_metadata: &mut Vec<arcana_hir::HirEmittedForewordMetadata>,
    registration_rows: &mut Vec<arcana_hir::HirForewordRegistrationRow>,
    warnings: &mut Vec<CheckWarning>,
    errors: &mut Vec<Diagnostic>,
) -> (HirImplDecl, Vec<HirSymbol>, Vec<HirImplDecl>, bool) {
    let mut changed = false;
    let mut appended_symbols = Vec::new();
    let mut appended_impls = Vec::new();
    let container_name = impl_container_name(&impl_decl);
    let old_methods = std::mem::take(&mut impl_decl.methods);
    let mut next_methods = Vec::with_capacity(old_methods.len());
    for method in old_methods {
        let (method, mut extra_symbols, mut extra_impls, method_changed) = transform_symbol(
            workspace,
            package,
            module_path,
            module_id,
            registry,
            executed,
            adapter_cache,
            method,
            "impl_method",
            Some("impl"),
            Some(&container_name),
            false,
            &[],
            emitted_metadata,
            registration_rows,
            warnings,
            errors,
        );
        changed |= method_changed;
        next_methods.push(method);
        appended_symbols.append(&mut extra_symbols);
        appended_impls.append(&mut extra_impls);
    }
    impl_decl.methods = next_methods;
    (impl_decl, appended_symbols, appended_impls, changed)
}

fn transform_module(
    workspace: &HirWorkspaceSummary,
    package: &HirWorkspacePackage,
    registry: &ForewordRegistry,
    executed: &mut BTreeSet<ExecutedTransformKey>,
    adapter_cache: &mut BTreeMap<String, ForewordAdapterResponse>,
    mut module: HirModuleSummary,
    warnings: &mut Vec<CheckWarning>,
    errors: &mut Vec<Diagnostic>,
) -> (HirModuleSummary, bool) {
    let module_path = package
        .module_path(&module.module_id)
        .cloned()
        .unwrap_or_else(|| package.root_dir.join("src").join("unknown.arc"));
    let mut changed = false;
    let mut pending_symbols = Vec::new();
    let mut pending_impls = Vec::new();
    let mut pending_emitted_metadata = std::mem::take(&mut module.emitted_foreword_metadata);
    let mut pending_registration_rows = std::mem::take(&mut module.foreword_registrations);

    let old_directives = std::mem::take(&mut module.directives);
    let mut next_directives = Vec::with_capacity(old_directives.len());
    for mut directive in old_directives {
        let target_kind = directive.kind.as_str().to_string();
        let target_public = directive.kind == arcana_hir::HirDirectiveKind::Reexport;
        let target_path = format!("{}:{}", module.module_id, directive.path.join("."));
        for app in directive.forewords.clone() {
            let target = AdapterTargetSnapshot {
                kind: target_kind.clone(),
                path: target_path.clone(),
                public: target_public,
                owner_kind: "directive".to_string(),
                owner_symbol: None,
                owner_directive: Some(build_adapter_directive_snapshot(&directive)),
                selected_field: None,
                selected_param: None,
                selected_method_name: None,
                container_kind: None,
                container_name: None,
            };
            if let Some(response) = maybe_execute_transform_app(
                workspace,
                package,
                &module_path,
                &module.module_id,
                registry,
                executed,
                adapter_cache,
                &app,
                &target_kind,
                &target_path,
                target,
                &lint_policy_with_inherited(
                    [lint_policy_layer_from_forewords(&directive.forewords)],
                    &[],
                ),
                warnings,
                errors,
            ) {
                apply_directive_transform_response(
                    &module.module_id,
                    &module_path,
                    &app,
                    &target_path,
                    target_public,
                    &mut directive,
                    response,
                    &mut pending_symbols,
                    &mut pending_impls,
                    &mut pending_emitted_metadata,
                    &mut pending_registration_rows,
                    &mut changed,
                    errors,
                );
            }
        }
        next_directives.push(directive);
    }
    module.directives = next_directives;

    let old_symbols = std::mem::take(&mut module.symbols);
    for symbol in old_symbols {
        let public = symbol.exported;
        let target_kind = symbol.kind.as_str().to_string();
        let (symbol, mut extra_symbols, mut extra_impls, symbol_changed) = transform_symbol(
            workspace,
            package,
            &module_path,
            &module.module_id,
            registry,
            executed,
            adapter_cache,
            symbol,
            &target_kind,
            None,
            None,
            public,
            &[],
            &mut pending_emitted_metadata,
            &mut pending_registration_rows,
            warnings,
            errors,
        );
        changed |= symbol_changed;
        pending_symbols.push(symbol);
        pending_symbols.append(&mut extra_symbols);
        pending_impls.append(&mut extra_impls);
    }
    module.symbols = pending_symbols;

    let old_impls = std::mem::take(&mut module.impls);
    let mut rebuilt_impls = Vec::new();
    for impl_decl in old_impls {
        let (impl_decl, mut extra_symbols, mut extra_impls, impl_changed) = transform_impl(
            workspace,
            package,
            &module_path,
            &module.module_id,
            registry,
            executed,
            adapter_cache,
            impl_decl,
            &mut pending_emitted_metadata,
            &mut pending_registration_rows,
            warnings,
            errors,
        );
        changed |= impl_changed;
        rebuilt_impls.push(impl_decl);
        module.symbols.append(&mut extra_symbols);
        rebuilt_impls.append(&mut extra_impls);
    }
    rebuilt_impls.append(&mut pending_impls);
    module.impls = rebuilt_impls;
    module.emitted_foreword_metadata = pending_emitted_metadata;
    module.foreword_registrations = pending_registration_rows;

    (module, changed)
}

fn apply_executable_foreword_transforms(
    mut workspace: HirWorkspaceSummary,
) -> Result<(HirWorkspaceSummary, Vec<CheckWarning>), String> {
    let mut executed = BTreeSet::<ExecutedTransformKey>::new();
    let mut adapter_cache = BTreeMap::<String, ForewordAdapterResponse>::new();
    let mut warnings = Vec::new();
    loop {
        let lookup_workspace = workspace.clone();
        let registry = build_foreword_registry(&lookup_workspace);
        if !registry.errors.is_empty() {
            return Err(render_diagnostics(registry.errors));
        }
        let mut errors = Vec::new();
        let mut pass_changed = false;
        for (package_id, package) in &mut workspace.packages {
            let Some(lookup_package) = lookup_workspace.package_by_id(package_id) else {
                continue;
            };
            let old_modules = std::mem::take(&mut package.summary.modules);
            let mut new_modules = Vec::with_capacity(old_modules.len());
            for module in old_modules {
                let (module, changed) = transform_module(
                    &lookup_workspace,
                    lookup_package,
                    &registry,
                    &mut executed,
                    &mut adapter_cache,
                    module,
                    &mut warnings,
                    &mut errors,
                );
                pass_changed |= changed;
                new_modules.push(module);
            }
            package.summary.modules = new_modules;
        }
        if !errors.is_empty() {
            return Err(render_diagnostics(errors));
        }
        if !pass_changed {
            break;
        }
    }
    Ok((workspace, warnings))
}

fn collect_symbol_executable_metadata(
    workspace: &HirWorkspaceSummary,
    package: &HirWorkspacePackage,
    module_path: &Path,
    module_id: &str,
    registry: &ForewordRegistry,
    adapter_cache: &mut BTreeMap<String, ForewordAdapterResponse>,
    symbol: &HirSymbol,
    target_kind: &str,
    container_kind: Option<&str>,
    container_name: Option<&str>,
    public: bool,
    inherited_lint_layers: &[LintPolicyLayer],
    emitted_metadata: &mut Vec<arcana_hir::HirEmittedForewordMetadata>,
    registration_rows: &mut Vec<arcana_hir::HirForewordRegistrationRow>,
    warnings: &mut Vec<CheckWarning>,
    errors: &mut Vec<Diagnostic>,
) {
    let target_path = target_path_for_symbol(module_id, &symbol.name, container_name);
    let symbol_lint_layer = lint_policy_layer_from_forewords(&symbol.forewords);

    for app in &symbol.forewords {
        let target = AdapterTargetSnapshot {
            kind: target_kind.to_string(),
            path: target_path.clone(),
            public,
            owner_kind: "symbol".to_string(),
            owner_symbol: Some(build_adapter_symbol_snapshot(symbol)),
            owner_directive: None,
            selected_field: None,
            selected_param: None,
            selected_method_name: None,
            container_kind: container_kind.map(ToString::to_string),
            container_name: container_name.map(ToString::to_string),
        };
        if let Some(response) = maybe_execute_metadata_app(
            workspace,
            package,
            module_path,
            module_id,
            registry,
            adapter_cache,
            app,
            target_kind,
            &target_path,
            target,
            &lint_policy_with_inherited([symbol_lint_layer.clone()], inherited_lint_layers),
            warnings,
            errors,
        ) {
            apply_metadata_response(
                module_path,
                app,
                public,
                response,
                emitted_metadata,
                registration_rows,
                errors,
            );
        }
    }

    for param in &symbol.params {
        let param_target_path =
            target_path_for_param(module_id, &symbol.name, &param.name, container_name);
        for app in &param.forewords {
            let target = AdapterTargetSnapshot {
                kind: "param".to_string(),
                path: param_target_path.clone(),
                public,
                owner_kind: "symbol".to_string(),
                owner_symbol: Some(build_adapter_symbol_snapshot(symbol)),
                owner_directive: None,
                selected_field: None,
                selected_param: Some(build_adapter_param_snapshot(param)),
                selected_method_name: None,
                container_kind: container_kind.map(ToString::to_string),
                container_name: container_name.map(ToString::to_string),
            };
            if let Some(response) = maybe_execute_metadata_app(
                workspace,
                package,
                module_path,
                module_id,
                registry,
                adapter_cache,
                app,
                "param",
                &param_target_path,
                target,
                &lint_policy_with_inherited(
                    [
                        lint_policy_layer_from_forewords(&param.forewords),
                        symbol_lint_layer.clone(),
                    ],
                    inherited_lint_layers,
                ),
                warnings,
                errors,
            ) {
                apply_metadata_response(
                    module_path,
                    app,
                    public,
                    response,
                    emitted_metadata,
                    registration_rows,
                    errors,
                );
            }
        }
    }

    match &symbol.body {
        HirSymbolBody::Record { fields } | HirSymbolBody::Object { fields, .. } => {
            for field in fields {
                let field_target_path =
                    target_path_for_field(module_id, &symbol.name, &field.name, container_name);
                for app in &field.forewords {
                    let target = AdapterTargetSnapshot {
                        kind: "field".to_string(),
                        path: field_target_path.clone(),
                        public,
                        owner_kind: "symbol".to_string(),
                        owner_symbol: Some(build_adapter_symbol_snapshot(symbol)),
                        owner_directive: None,
                        selected_field: Some(build_adapter_field_snapshot(field)),
                        selected_param: None,
                        selected_method_name: None,
                        container_kind: container_kind.map(ToString::to_string),
                        container_name: container_name.map(ToString::to_string),
                    };
                    if let Some(response) = maybe_execute_metadata_app(
                        workspace,
                        package,
                        module_path,
                        module_id,
                        registry,
                        adapter_cache,
                        app,
                        "field",
                        &field_target_path,
                        target,
                        &lint_policy_with_inherited(
                            [
                                lint_policy_layer_from_forewords(&field.forewords),
                                symbol_lint_layer.clone(),
                            ],
                            inherited_lint_layers,
                        ),
                        warnings,
                        errors,
                    ) {
                        apply_metadata_response(
                            module_path,
                            app,
                            public,
                            response,
                            emitted_metadata,
                            registration_rows,
                            errors,
                        );
                    }
                }
            }
        }
        _ => {}
    }

    let child_inherited_layers = std::iter::once(symbol_lint_layer)
        .chain(inherited_lint_layers.iter().cloned())
        .collect::<Vec<_>>();
    match &symbol.body {
        HirSymbolBody::Object { methods, .. } => {
            for method in methods {
                collect_symbol_executable_metadata(
                    workspace,
                    package,
                    module_path,
                    module_id,
                    registry,
                    adapter_cache,
                    method,
                    "impl_method",
                    Some("object"),
                    Some(&symbol.name),
                    public,
                    &child_inherited_layers,
                    emitted_metadata,
                    registration_rows,
                    warnings,
                    errors,
                );
            }
        }
        HirSymbolBody::Trait { methods, .. } => {
            for method in methods {
                collect_symbol_executable_metadata(
                    workspace,
                    package,
                    module_path,
                    module_id,
                    registry,
                    adapter_cache,
                    method,
                    "trait_method",
                    Some("trait"),
                    Some(&symbol.name),
                    public,
                    &child_inherited_layers,
                    emitted_metadata,
                    registration_rows,
                    warnings,
                    errors,
                );
            }
        }
        _ => {}
    }
}

fn collect_impl_executable_metadata(
    workspace: &HirWorkspaceSummary,
    package: &HirWorkspacePackage,
    module_path: &Path,
    module_id: &str,
    registry: &ForewordRegistry,
    adapter_cache: &mut BTreeMap<String, ForewordAdapterResponse>,
    impl_decl: &HirImplDecl,
    emitted_metadata: &mut Vec<arcana_hir::HirEmittedForewordMetadata>,
    registration_rows: &mut Vec<arcana_hir::HirForewordRegistrationRow>,
    warnings: &mut Vec<CheckWarning>,
    errors: &mut Vec<Diagnostic>,
) {
    let container_name = impl_container_name(impl_decl);
    for method in &impl_decl.methods {
        collect_symbol_executable_metadata(
            workspace,
            package,
            module_path,
            module_id,
            registry,
            adapter_cache,
            method,
            "impl_method",
            Some("impl"),
            Some(&container_name),
            false,
            &[],
            emitted_metadata,
            registration_rows,
            warnings,
            errors,
        );
    }
}

fn collect_module_executable_metadata(
    workspace: &HirWorkspaceSummary,
    package: &HirWorkspacePackage,
    registry: &ForewordRegistry,
    adapter_cache: &mut BTreeMap<String, ForewordAdapterResponse>,
    mut module: HirModuleSummary,
    warnings: &mut Vec<CheckWarning>,
    errors: &mut Vec<Diagnostic>,
) -> HirModuleSummary {
    let module_path = package
        .module_path(&module.module_id)
        .cloned()
        .unwrap_or_else(|| package.root_dir.join("src").join("unknown.arc"));
    let mut pending_emitted_metadata = std::mem::take(&mut module.emitted_foreword_metadata);
    let mut pending_registration_rows = std::mem::take(&mut module.foreword_registrations);

    for directive in &module.directives {
        let target_kind = directive.kind.as_str().to_string();
        let target_public = directive.kind == arcana_hir::HirDirectiveKind::Reexport;
        let target_path = format!("{}:{}", module.module_id, directive.path.join("."));
        for app in &directive.forewords {
            let target = AdapterTargetSnapshot {
                kind: target_kind.clone(),
                path: target_path.clone(),
                public: target_public,
                owner_kind: "directive".to_string(),
                owner_symbol: None,
                owner_directive: Some(build_adapter_directive_snapshot(directive)),
                selected_field: None,
                selected_param: None,
                selected_method_name: None,
                container_kind: None,
                container_name: None,
            };
            if let Some(response) = maybe_execute_metadata_app(
                workspace,
                package,
                &module_path,
                &module.module_id,
                registry,
                adapter_cache,
                app,
                &target_kind,
                &target_path,
                target,
                &lint_policy_with_inherited(
                    [lint_policy_layer_from_forewords(&directive.forewords)],
                    &[],
                ),
                warnings,
                errors,
            ) {
                apply_metadata_response(
                    &module_path,
                    app,
                    target_public,
                    response,
                    &mut pending_emitted_metadata,
                    &mut pending_registration_rows,
                    errors,
                );
            }
        }
    }

    for symbol in &module.symbols {
        collect_symbol_executable_metadata(
            workspace,
            package,
            &module_path,
            &module.module_id,
            registry,
            adapter_cache,
            symbol,
            symbol.kind.as_str(),
            None,
            None,
            symbol.exported,
            &[],
            &mut pending_emitted_metadata,
            &mut pending_registration_rows,
            warnings,
            errors,
        );
    }

    for impl_decl in &module.impls {
        collect_impl_executable_metadata(
            workspace,
            package,
            &module_path,
            &module.module_id,
            registry,
            adapter_cache,
            impl_decl,
            &mut pending_emitted_metadata,
            &mut pending_registration_rows,
            warnings,
            errors,
        );
    }

    module.emitted_foreword_metadata = pending_emitted_metadata;
    module.foreword_registrations = pending_registration_rows;
    module
}

fn apply_executable_foreword_metadata(
    mut workspace: HirWorkspaceSummary,
) -> Result<(HirWorkspaceSummary, Vec<CheckWarning>), String> {
    let lookup_workspace = workspace.clone();
    let registry = build_foreword_registry(&lookup_workspace);
    if !registry.errors.is_empty() {
        return Err(render_diagnostics(registry.errors));
    }
    let mut adapter_cache = BTreeMap::<String, ForewordAdapterResponse>::new();
    let mut warnings = Vec::new();
    let mut errors = Vec::new();
    for (package_id, package) in &mut workspace.packages {
        let Some(lookup_package) = lookup_workspace.package_by_id(package_id) else {
            continue;
        };
        let old_modules = std::mem::take(&mut package.summary.modules);
        let mut new_modules = Vec::with_capacity(old_modules.len());
        for module in old_modules {
            let module = collect_module_executable_metadata(
                &lookup_workspace,
                lookup_package,
                &registry,
                &mut adapter_cache,
                module,
                &mut warnings,
                &mut errors,
            );
            new_modules.push(module);
        }
        package.summary.modules = new_modules;
    }
    if !errors.is_empty() {
        return Err(render_diagnostics(errors));
    }
    Ok((workspace, warnings))
}

fn deprecated_message(symbol: &HirSymbol) -> Option<String> {
    let foreword = symbol
        .forewords
        .iter()
        .find(|foreword| foreword.path.len() == 1 && foreword.name == "deprecated")?;
    foreword.args.first().map(|arg| match &arg.typed_value {
        arcana_hir::HirForewordArgValue::Str(value) => value.clone(),
        _ => arg.value.trim().trim_matches('"').to_string(),
    })
}

#[derive(Clone, Debug, Default)]
struct LintPolicyLayer {
    allow: Vec<String>,
    deny: Vec<String>,
}

#[derive(Clone, Debug, Default)]
struct LintPolicy {
    layers: Vec<LintPolicyLayer>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum LintMatchSpecificity {
    Namespace = 0,
    Exact = 1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum LintDecision {
    Allow,
    Deny,
}

fn lint_policy_layer_from_forewords(forewords: &[arcana_hir::HirForewordApp]) -> LintPolicyLayer {
    let mut policy = LintPolicyLayer::default();
    for foreword in forewords {
        if foreword.path.len() != 1 || (foreword.name != "allow" && foreword.name != "deny") {
            continue;
        }
        let target = if foreword.name == "allow" {
            &mut policy.allow
        } else {
            &mut policy.deny
        };
        target.extend(foreword.args.iter().map(|arg| arg.typed_value.render()));
    }
    policy
}

fn lint_policy_from_layers(layers: Vec<LintPolicyLayer>) -> LintPolicy {
    LintPolicy { layers }
}

fn lint_policy_with_inherited<I>(layers: I, inherited: &[LintPolicyLayer]) -> LintPolicy
where
    I: IntoIterator<Item = LintPolicyLayer>,
{
    let mut merged = layers.into_iter().collect::<Vec<_>>();
    merged.extend(inherited.iter().cloned());
    lint_policy_from_layers(merged)
}

fn lint_match_specificity(pattern: &str, lint_name: &str) -> Option<LintMatchSpecificity> {
    if pattern == lint_name {
        Some(LintMatchSpecificity::Exact)
    } else if lint_name
        .strip_prefix(pattern)
        .is_some_and(|tail| tail.starts_with('.'))
    {
        Some(LintMatchSpecificity::Namespace)
    } else {
        None
    }
}

fn lint_decision(policy: &LintPolicy, lint_name: &str) -> Option<LintDecision> {
    let mut best: Option<(LintMatchSpecificity, std::cmp::Reverse<usize>, LintDecision)> = None;
    for (layer_index, layer) in policy.layers.iter().enumerate() {
        for pattern in &layer.allow {
            if let Some(specificity) = lint_match_specificity(pattern, lint_name) {
                let candidate = (
                    specificity,
                    std::cmp::Reverse(layer_index),
                    LintDecision::Allow,
                );
                if best.is_none_or(|current| candidate > current) {
                    best = Some(candidate);
                }
            }
        }
        for pattern in &layer.deny {
            if let Some(specificity) = lint_match_specificity(pattern, lint_name) {
                let candidate = (
                    specificity,
                    std::cmp::Reverse(layer_index),
                    LintDecision::Deny,
                );
                if best.is_none_or(|current| candidate > current) {
                    best = Some(candidate);
                }
            }
        }
    }
    best.map(|(_, _, decision)| decision)
}

fn lint_is_allowed(policy: &LintPolicy, lint_name: &str) -> bool {
    lint_decision(policy, lint_name) == Some(LintDecision::Allow)
}

fn lint_is_denied(policy: &LintPolicy, lint_name: &str) -> bool {
    lint_decision(policy, lint_name) == Some(LintDecision::Deny)
}

fn maybe_emit_basic_foreword_warning(
    export: &ResolvedForewordExport,
    module_path: &Path,
    app: &arcana_hir::HirForewordApp,
    policy: &LintPolicy,
    warnings: &mut Vec<CheckWarning>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if export.is_builtin() || export.definition.tier != arcana_hir::HirForewordTier::Basic {
        return;
    }
    let Some(lint_name) = export.definition.diagnostic_namespace.as_deref() else {
        return;
    };
    let message = format!(
        "basic foreword `#{}` is active on this target",
        app.path.join(".")
    );
    if lint_is_allowed(policy, lint_name) {
        return;
    }
    if lint_is_denied(policy, lint_name) {
        diagnostics.push(Diagnostic {
            path: module_path.to_path_buf(),
            line: app.span.line,
            column: app.span.column,
            message,
        });
    } else {
        warnings.push(CheckWarning {
            path: module_path.to_path_buf(),
            line: app.span.line,
            column: app.span.column,
            message,
        });
    }
}

fn normalize_foreword_conflict_name(
    package: &HirWorkspacePackage,
    registry: &ForewordRegistry,
    path: &[String],
) -> String {
    if path.len() < 2 {
        return path.join(".");
    }
    let probe = arcana_hir::HirForewordApp {
        name: path.last().cloned().unwrap_or_default(),
        path: path.to_vec(),
        args: Vec::new(),
        span: Span { line: 0, column: 0 },
    };
    resolve_user_foreword_export(package, &probe, registry)
        .map(|export| export.exposed_name.join("."))
        .unwrap_or_else(|| path.join("."))
}

fn validate_foreword_target_contracts(
    package: &HirWorkspacePackage,
    module_path: &Path,
    apps: &[arcana_hir::HirForewordApp],
    registry: &ForewordRegistry,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let resolved_apps = apps
        .iter()
        .filter_map(|app| {
            let export = resolve_user_foreword_export(package, app, registry)?;
            Some((app, export))
        })
        .collect::<Vec<_>>();

    let mut seen = BTreeMap::<
        String,
        (
            &arcana_hir::HirForewordDefinition,
            &arcana_hir::HirForewordApp,
        ),
    >::new();
    for (app, export) in &resolved_apps {
        let key = export.exposed_name.join(".");
        if let Some((_first_def, _first_app)) = seen.get(&key) {
            if !export.definition.repeatable {
                diagnostics.push(Diagnostic {
                    path: module_path.to_path_buf(),
                    line: app.span.line,
                    column: app.span.column,
                    message: format!("`#{}` is not repeatable on the same target", key),
                });
            }
        } else {
            seen.insert(key, (&export.definition, app));
        }
    }

    let attached = resolved_apps
        .iter()
        .map(|(app, export)| (export.exposed_name.join("."), *app, *export))
        .collect::<Vec<_>>();
    let mut emitted = BTreeSet::<(String, String)>::new();
    for (attached_name, app, export) in &attached {
        for conflict_path in &export.definition.conflicts {
            let conflict_name = normalize_foreword_conflict_name(package, registry, conflict_path);
            if attached
                .iter()
                .any(|(other_name, _, _)| other_name == &conflict_name)
                && emitted.insert((attached_name.clone(), conflict_name.clone()))
            {
                diagnostics.push(Diagnostic {
                    path: module_path.to_path_buf(),
                    line: app.span.line,
                    column: app.span.column,
                    message: format!(
                        "`#{}` conflicts with `#{}` on the same target",
                        attached_name, conflict_name
                    ),
                });
            }
        }
    }
}

fn validate_foreword_apps_for_target(
    package: &HirWorkspacePackage,
    module_path: &Path,
    module_id: &str,
    apps: &[arcana_hir::HirForewordApp],
    target_kind: &str,
    target_path: &str,
    target_public: bool,
    target_generated_by: Option<&arcana_hir::HirGeneratedByForeword>,
    registry: &ForewordRegistry,
    inherited_lint_layers: &[LintPolicyLayer],
    warnings: &mut Vec<CheckWarning>,
    diagnostics: &mut Vec<Diagnostic>,
    index: &mut Vec<ForewordIndexEntry>,
) {
    validate_foreword_target_contracts(package, module_path, apps, registry, diagnostics);
    let policy = lint_policy_with_inherited(
        [lint_policy_layer_from_forewords(apps)],
        inherited_lint_layers,
    );
    for app in apps {
        let Some(export) = resolve_foreword_export(package, app, registry) else {
            diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: app.span.line,
                column: app.span.column,
                message: format!("unresolved foreword `#{}`", app.path.join(".")),
            });
            continue;
        };
        if !target_allowed_for_definition(&export.definition, target_kind) {
            diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: app.span.line,
                column: app.span.column,
                message: format!(
                    "`#{}` is not valid for {} targets",
                    app.path.join("."),
                    target_kind
                ),
            });
            continue;
        }
        if !export.is_builtin()
            && let Err(message) = validate_user_foreword_payload(app, &export.definition)
        {
            diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: app.span.line,
                column: app.span.column,
                message,
            });
            continue;
        }
        maybe_emit_basic_foreword_warning(export, module_path, app, &policy, warnings, diagnostics);
        if export.definition.tier == arcana_hir::HirForewordTier::Executable {
            if !dependency_allows_executable_forewords(package, &export.exposed_package_id) {
                diagnostics.push(Diagnostic {
                    path: module_path.to_path_buf(),
                    line: app.span.line,
                    column: app.span.column,
                    message: format!(
                        "executable foreword `#{}` requires `executable_forewords = true` on the dependency that exports it",
                        app.path.join(".")
                    ),
                });
                continue;
            }
            let Some(handler) = &export.handler else {
                diagnostics.push(Diagnostic {
                    path: module_path.to_path_buf(),
                    line: app.span.line,
                    column: app.span.column,
                    message: format!(
                        "executable foreword `#{}` is missing a handler binding",
                        app.path.join(".")
                    ),
                });
                continue;
            };
            if handler.protocol != "stdio-v1" {
                diagnostics.push(Diagnostic {
                    path: module_path.to_path_buf(),
                    line: app.span.line,
                    column: app.span.column,
                    message: format!(
                        "foreword handler `{}` uses unsupported protocol `{}`",
                        handler.qualified_name.join("."),
                        handler.protocol
                    ),
                });
                continue;
            }
        }
        index.push(ForewordIndexEntry {
            entry_kind: "attached".to_string(),
            qualified_name: export.exposed_name.join("."),
            package_id: package.package_id.clone(),
            module_id: module_id.to_string(),
            target_kind: target_kind.to_string(),
            target_path: target_path.to_string(),
            retention: export.definition.retention.as_str().to_string(),
            args: render_foreword_args(app),
            public: target_public,
            generated_by: target_generated_by.map(lower_generated_by),
        });
    }
}

fn generated_by_for_attached_foreword(
    app: &arcana_hir::HirForewordApp,
    export: &ResolvedForewordExport,
    target_kind: &str,
    target_path: &str,
) -> arcana_hir::HirGeneratedByForeword {
    arcana_hir::HirGeneratedByForeword {
        applied_name: app.path.join("."),
        resolved_name: export.exposed_name.join("."),
        provider_package_id: export.provider_package_id.clone(),
        owner_kind: target_kind.to_string(),
        owner_path: target_path.to_string(),
        retention: export.definition.retention,
        args: app.args.clone(),
    }
}

fn basic_registration_rows_for_app(
    app: &arcana_hir::HirForewordApp,
    export: &ResolvedForewordExport,
    target_kind: &str,
    target_path: &str,
    target_public: bool,
) -> Vec<arcana_hir::HirForewordRegistrationRow> {
    let generated_by = generated_by_for_attached_foreword(app, export, target_kind, target_path);
    let namespace = export.exposed_name.join(".");
    if app.args.is_empty() {
        return vec![arcana_hir::HirForewordRegistrationRow {
            namespace,
            key: "present".to_string(),
            value: "true".to_string(),
            target_kind: target_kind.to_string(),
            target_path: target_path.to_string(),
            public: target_public,
            generated_by,
        }];
    }
    app.args
        .iter()
        .enumerate()
        .map(|(index, arg)| arcana_hir::HirForewordRegistrationRow {
            namespace: namespace.clone(),
            key: arg.name.clone().unwrap_or_else(|| format!("arg{index}")),
            value: arg.typed_value.render(),
            target_kind: target_kind.to_string(),
            target_path: target_path.to_string(),
            public: target_public,
            generated_by: generated_by.clone(),
        })
        .collect()
}

fn append_basic_registration_rows_for_target(
    package: &HirWorkspacePackage,
    apps: &[arcana_hir::HirForewordApp],
    target_kind: &str,
    target_path: &str,
    target_public: bool,
    registry: &ForewordRegistry,
    rows: &mut Vec<arcana_hir::HirForewordRegistrationRow>,
) {
    for app in apps {
        if app.path.len() < 2 {
            continue;
        }
        let Some(export) = resolve_user_foreword_export(package, app, registry) else {
            continue;
        };
        if export.definition.tier != arcana_hir::HirForewordTier::Basic
            || !target_allowed_for_definition(&export.definition, target_kind)
            || validate_user_foreword_payload(app, &export.definition).is_err()
        {
            continue;
        }
        rows.extend(basic_registration_rows_for_app(
            app,
            export,
            target_kind,
            target_path,
            target_public,
        ));
    }
}

fn append_basic_registration_rows_for_symbol(
    package: &HirWorkspacePackage,
    module_id: &str,
    symbol: &HirSymbol,
    target_kind: &str,
    public: bool,
    registry: &ForewordRegistry,
    rows: &mut Vec<arcana_hir::HirForewordRegistrationRow>,
) {
    let symbol_target_path = format!("{}.{}", module_id, symbol.name);
    append_basic_registration_rows_for_target(
        package,
        &symbol.forewords,
        target_kind,
        &symbol_target_path,
        public,
        registry,
        rows,
    );
    for param in &symbol.params {
        append_basic_registration_rows_for_target(
            package,
            &param.forewords,
            "param",
            &format!("{}.{}({})", module_id, symbol.name, param.name),
            public,
            registry,
            rows,
        );
    }
    match &symbol.body {
        HirSymbolBody::Record { fields } => {
            for field in fields {
                append_basic_registration_rows_for_target(
                    package,
                    &field.forewords,
                    "field",
                    &format!("{}.{}.{}", module_id, symbol.name, field.name),
                    public,
                    registry,
                    rows,
                );
            }
        }
        HirSymbolBody::Object { fields, methods } => {
            for field in fields {
                append_basic_registration_rows_for_target(
                    package,
                    &field.forewords,
                    "field",
                    &format!("{}.{}.{}", module_id, symbol.name, field.name),
                    public,
                    registry,
                    rows,
                );
            }
            for method in methods {
                append_basic_registration_rows_for_symbol(
                    package,
                    module_id,
                    method,
                    "impl_method",
                    public,
                    registry,
                    rows,
                );
            }
        }
        HirSymbolBody::Trait { methods, .. } => {
            for method in methods {
                append_basic_registration_rows_for_symbol(
                    package,
                    module_id,
                    method,
                    "trait_method",
                    public,
                    registry,
                    rows,
                );
            }
        }
        _ => {}
    }
}

fn populate_basic_foreword_registrations(
    mut workspace: HirWorkspaceSummary,
) -> HirWorkspaceSummary {
    let registry = build_foreword_registry(&workspace);
    if !registry.errors.is_empty() {
        return workspace;
    }
    let lookup_workspace = workspace.clone();
    for (package_id, package) in &mut workspace.packages {
        let Some(lookup_package) = lookup_workspace.package_by_id(package_id) else {
            continue;
        };
        for module in &mut package.summary.modules {
            let mut generated_rows = Vec::new();
            for directive in &module.directives {
                append_basic_registration_rows_for_target(
                    lookup_package,
                    &directive.forewords,
                    directive.kind.as_str(),
                    &format!("{}:{}", module.module_id, directive.path.join(".")),
                    directive.kind == arcana_hir::HirDirectiveKind::Reexport,
                    &registry,
                    &mut generated_rows,
                );
            }
            for symbol in &module.symbols {
                append_basic_registration_rows_for_symbol(
                    lookup_package,
                    &module.module_id,
                    symbol,
                    symbol.kind.as_str(),
                    symbol.exported,
                    &registry,
                    &mut generated_rows,
                );
            }
            for impl_decl in &module.impls {
                for method in &impl_decl.methods {
                    append_basic_registration_rows_for_symbol(
                        lookup_package,
                        &module.module_id,
                        method,
                        "impl_method",
                        false,
                        &registry,
                        &mut generated_rows,
                    );
                }
            }
            module.foreword_registrations.extend(generated_rows);
        }
    }
    workspace
}

fn collect_deprecated_call_warnings_in_expr(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    type_scope: &TypeScope,
    scope: &ValueScope,
    expr: &HirExpr,
    span: Span,
    policy: &LintPolicy,
    warnings: &mut Vec<CheckWarning>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match expr {
        HirExpr::QualifiedPhrase {
            subject,
            args,
            qualifier_kind,
            qualifier,
            ..
        } => {
            if let Some(symbol) = resolve_qualified_phrase_target_symbol(
                workspace,
                resolved_module,
                type_scope,
                scope,
                subject,
                *qualifier_kind,
                qualifier,
            ) && let Some(message) = deprecated_message(symbol)
            {
                let lint_name = "deprecated_use";
                if !lint_is_allowed(policy, lint_name) {
                    let rendered = format!("use of deprecated `{}`: {}", symbol.name, message);
                    if lint_is_denied(policy, lint_name) {
                        diagnostics.push(Diagnostic {
                            path: module_path.to_path_buf(),
                            line: span.line,
                            column: span.column,
                            message: rendered,
                        });
                    } else {
                        warnings.push(CheckWarning {
                            path: module_path.to_path_buf(),
                            line: span.line,
                            column: span.column,
                            message: rendered,
                        });
                    }
                }
            }
            collect_deprecated_call_warnings_in_expr(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                subject,
                span,
                policy,
                warnings,
                diagnostics,
            );
            for arg in args {
                match arg {
                    arcana_hir::HirPhraseArg::Positional(expr)
                    | arcana_hir::HirPhraseArg::Named { value: expr, .. } => {
                        collect_deprecated_call_warnings_in_expr(
                            workspace,
                            resolved_module,
                            module_path,
                            type_scope,
                            scope,
                            expr,
                            span,
                            policy,
                            warnings,
                            diagnostics,
                        );
                    }
                }
            }
        }
        HirExpr::GenericApply { expr, .. }
        | HirExpr::Await { expr }
        | HirExpr::MemberAccess { expr, .. }
        | HirExpr::Unary { expr, .. } => collect_deprecated_call_warnings_in_expr(
            workspace,
            resolved_module,
            module_path,
            type_scope,
            scope,
            expr,
            span,
            policy,
            warnings,
            diagnostics,
        ),
        HirExpr::Pair { left, right } | HirExpr::Binary { left, right, .. } => {
            collect_deprecated_call_warnings_in_expr(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                left,
                span,
                policy,
                warnings,
                diagnostics,
            );
            collect_deprecated_call_warnings_in_expr(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                right,
                span,
                policy,
                warnings,
                diagnostics,
            );
        }
        HirExpr::CollectionLiteral { items } => {
            for item in items {
                collect_deprecated_call_warnings_in_expr(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    item,
                    span,
                    policy,
                    warnings,
                    diagnostics,
                );
            }
        }
        HirExpr::ConstructRegion(region) => {
            collect_deprecated_call_warnings_in_expr(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                &region.target,
                span,
                policy,
                warnings,
                diagnostics,
            );
            for line in &region.lines {
                collect_deprecated_call_warnings_in_expr(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    &line.value,
                    span,
                    policy,
                    warnings,
                    diagnostics,
                );
            }
            if let Some(arcana_hir::HirConstructDestination::Place { target }) = &region.destination
            {
                let expr = assign_target_to_expr(target);
                collect_deprecated_call_warnings_in_expr(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    &expr,
                    span,
                    policy,
                    warnings,
                    diagnostics,
                );
            }
        }
        HirExpr::RecordRegion(region) => {
            collect_deprecated_call_warnings_in_expr(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                &region.target,
                span,
                policy,
                warnings,
                diagnostics,
            );
            if let Some(base) = &region.base {
                collect_deprecated_call_warnings_in_expr(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    base,
                    span,
                    policy,
                    warnings,
                    diagnostics,
                );
            }
            for line in &region.lines {
                collect_deprecated_call_warnings_in_expr(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    &line.value,
                    span,
                    policy,
                    warnings,
                    diagnostics,
                );
            }
            if let Some(arcana_hir::HirConstructDestination::Place { target }) = &region.destination
            {
                let expr = assign_target_to_expr(target);
                collect_deprecated_call_warnings_in_expr(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    &expr,
                    span,
                    policy,
                    warnings,
                    diagnostics,
                );
            }
        }
        HirExpr::Chain { steps, .. } => {
            for step in steps {
                collect_deprecated_call_warnings_in_expr(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    &step.stage,
                    span,
                    policy,
                    warnings,
                    diagnostics,
                );
                for arg in &step.bind_args {
                    collect_deprecated_call_warnings_in_expr(
                        workspace,
                        resolved_module,
                        module_path,
                        type_scope,
                        scope,
                        arg,
                        span,
                        policy,
                        warnings,
                        diagnostics,
                    );
                }
            }
        }
        HirExpr::Match { subject, arms } => {
            collect_deprecated_call_warnings_in_expr(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                subject,
                span,
                policy,
                warnings,
                diagnostics,
            );
            for arm in arms {
                collect_deprecated_call_warnings_in_expr(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    &arm.value,
                    span,
                    policy,
                    warnings,
                    diagnostics,
                );
            }
        }
        HirExpr::MemoryPhrase {
            arena,
            init_args,
            constructor,
            attached,
            ..
        } => {
            collect_deprecated_call_warnings_in_expr(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                arena,
                span,
                policy,
                warnings,
                diagnostics,
            );
            for arg in init_args {
                match arg {
                    arcana_hir::HirPhraseArg::Positional(expr)
                    | arcana_hir::HirPhraseArg::Named { value: expr, .. } => {
                        collect_deprecated_call_warnings_in_expr(
                            workspace,
                            resolved_module,
                            module_path,
                            type_scope,
                            scope,
                            expr,
                            span,
                            policy,
                            warnings,
                            diagnostics,
                        );
                    }
                }
            }
            collect_deprecated_call_warnings_in_expr(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                constructor,
                span,
                policy,
                warnings,
                diagnostics,
            );
            for attachment in attached {
                match attachment {
                    HirHeaderAttachment::Named { value, .. }
                    | HirHeaderAttachment::Chain { expr: value, .. } => {
                        collect_deprecated_call_warnings_in_expr(
                            workspace,
                            resolved_module,
                            module_path,
                            type_scope,
                            scope,
                            value,
                            span,
                            policy,
                            warnings,
                            diagnostics,
                        );
                    }
                }
            }
        }
        HirExpr::Index { expr, index } => {
            collect_deprecated_call_warnings_in_expr(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                expr,
                span,
                policy,
                warnings,
                diagnostics,
            );
            collect_deprecated_call_warnings_in_expr(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                index,
                span,
                policy,
                warnings,
                diagnostics,
            );
        }
        HirExpr::Slice {
            expr, start, end, ..
        } => {
            collect_deprecated_call_warnings_in_expr(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                expr,
                span,
                policy,
                warnings,
                diagnostics,
            );
            if let Some(start) = start {
                collect_deprecated_call_warnings_in_expr(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    start,
                    span,
                    policy,
                    warnings,
                    diagnostics,
                );
            }
            if let Some(end) = end {
                collect_deprecated_call_warnings_in_expr(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    end,
                    span,
                    policy,
                    warnings,
                    diagnostics,
                );
            }
        }
        HirExpr::Range { start, end, .. } => {
            if let Some(start) = start {
                collect_deprecated_call_warnings_in_expr(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    start,
                    span,
                    policy,
                    warnings,
                    diagnostics,
                );
            }
            if let Some(end) = end {
                collect_deprecated_call_warnings_in_expr(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    end,
                    span,
                    policy,
                    warnings,
                    diagnostics,
                );
            }
        }
        HirExpr::Path { .. }
        | HirExpr::BoolLiteral { .. }
        | HirExpr::IntLiteral { .. }
        | HirExpr::StrLiteral { .. } => {}
    }
}

fn collect_deprecated_call_warnings_in_statements(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    type_scope: &TypeScope,
    scope: &mut ValueScope,
    statements: &[HirStatement],
    policy: &LintPolicy,
    warnings: &mut Vec<CheckWarning>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for statement in statements {
        match &statement.kind {
            HirStatementKind::Let {
                mutable,
                name,
                value,
            } => {
                collect_deprecated_call_warnings_in_expr(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    value,
                    statement.span,
                    policy,
                    warnings,
                    diagnostics,
                );
                let ty =
                    infer_expr_value_type(workspace, resolved_module, type_scope, scope, value);
                let _ = bind_pattern_into_scope(
                    workspace,
                    resolved_module,
                    type_scope,
                    scope,
                    name,
                    *mutable,
                    ty,
                );
            }
            HirStatementKind::Return { value } => {
                if let Some(value) = value {
                    collect_deprecated_call_warnings_in_expr(
                        workspace,
                        resolved_module,
                        module_path,
                        type_scope,
                        scope,
                        value,
                        statement.span,
                        policy,
                        warnings,
                        diagnostics,
                    );
                }
            }
            HirStatementKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                collect_deprecated_call_warnings_in_expr(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    condition,
                    statement.span,
                    policy,
                    warnings,
                    diagnostics,
                );
                let mut then_scope = scope.clone();
                collect_deprecated_call_warnings_in_statements(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    &mut then_scope,
                    then_branch,
                    policy,
                    warnings,
                    diagnostics,
                );
                if let Some(else_branch) = else_branch {
                    let mut else_scope = scope.clone();
                    collect_deprecated_call_warnings_in_statements(
                        workspace,
                        resolved_module,
                        module_path,
                        type_scope,
                        &mut else_scope,
                        else_branch,
                        policy,
                        warnings,
                        diagnostics,
                    );
                }
            }
            HirStatementKind::While { condition, body } => {
                collect_deprecated_call_warnings_in_expr(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    condition,
                    statement.span,
                    policy,
                    warnings,
                    diagnostics,
                );
                let mut body_scope = scope.clone();
                collect_deprecated_call_warnings_in_statements(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    &mut body_scope,
                    body,
                    policy,
                    warnings,
                    diagnostics,
                );
            }
            HirStatementKind::For {
                binding,
                iterable,
                body,
            } => {
                collect_deprecated_call_warnings_in_expr(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    iterable,
                    statement.span,
                    policy,
                    warnings,
                    diagnostics,
                );
                let mut body_scope = scope.clone();
                let iterable_binding_ty = infer_iterable_binding_type(
                    workspace,
                    resolved_module,
                    type_scope,
                    scope,
                    iterable,
                );
                let _ = bind_pattern_into_scope(
                    workspace,
                    resolved_module,
                    type_scope,
                    &mut body_scope,
                    binding,
                    false,
                    iterable_binding_ty,
                );
                collect_deprecated_call_warnings_in_statements(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    &mut body_scope,
                    body,
                    policy,
                    warnings,
                    diagnostics,
                );
            }
            HirStatementKind::Defer { expr } | HirStatementKind::Expr { expr } => {
                collect_deprecated_call_warnings_in_expr(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    expr,
                    statement.span,
                    policy,
                    warnings,
                    diagnostics,
                )
            }
            HirStatementKind::Assign { value, .. } => collect_deprecated_call_warnings_in_expr(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                value,
                statement.span,
                policy,
                warnings,
                diagnostics,
            ),
            HirStatementKind::Recycle { .. }
            | HirStatementKind::Bind { .. }
            | HirStatementKind::Record(_)
            | HirStatementKind::Construct(_)
            | HirStatementKind::MemorySpec(_)
            | HirStatementKind::Break
            | HirStatementKind::Continue => {}
        }
    }
}

fn push_generated_foreword_index_entry(
    package: &HirWorkspacePackage,
    module_id: &str,
    target_kind: &str,
    target_path: String,
    public: bool,
    generated_by: &arcana_hir::HirGeneratedByForeword,
    index: &mut Vec<ForewordIndexEntry>,
) {
    index.push(ForewordIndexEntry {
        entry_kind: "generated".to_string(),
        qualified_name: generated_by.resolved_name.clone(),
        package_id: package.package_id.clone(),
        module_id: module_id.to_string(),
        target_kind: target_kind.to_string(),
        target_path,
        retention: generated_by.retention.as_str().to_string(),
        args: render_hir_foreword_args(&generated_by.args),
        public,
        generated_by: Some(lower_generated_by(generated_by)),
    });
}

fn push_emitted_foreword_index_entry(
    package: &HirWorkspacePackage,
    module_id: &str,
    entry: &arcana_hir::HirEmittedForewordMetadata,
    index: &mut Vec<ForewordIndexEntry>,
) {
    index.push(ForewordIndexEntry {
        entry_kind: "emitted".to_string(),
        qualified_name: entry.qualified_name.clone(),
        package_id: package.package_id.clone(),
        module_id: module_id.to_string(),
        target_kind: entry.target_kind.clone(),
        target_path: entry.target_path.clone(),
        retention: entry.retention.as_str().to_string(),
        args: render_hir_foreword_args(&entry.args),
        public: entry.public,
        generated_by: Some(lower_generated_by(&entry.generated_by)),
    });
}

fn lower_registration_row(row: &arcana_hir::HirForewordRegistrationRow) -> ForewordRegistrationRow {
    ForewordRegistrationRow {
        namespace: row.namespace.clone(),
        key: row.key.clone(),
        value: row.value.clone(),
        target_kind: row.target_kind.clone(),
        target_path: row.target_path.clone(),
        public: row.public,
        generated_by: lower_generated_by(&row.generated_by),
    }
}

fn validate_symbol_declared_forewords(
    package: &HirWorkspacePackage,
    module: &HirModuleSummary,
    symbol: &HirSymbol,
    target_kind: &str,
    public: bool,
    inherited_generated_by: Option<&arcana_hir::HirGeneratedByForeword>,
    registry: &ForewordRegistry,
    inherited_lint_layers: &[LintPolicyLayer],
    warnings: &mut Vec<CheckWarning>,
    diagnostics: &mut Vec<Diagnostic>,
    index: &mut Vec<ForewordIndexEntry>,
) {
    let module_path = package
        .module_path(&module.module_id)
        .cloned()
        .unwrap_or_else(|| package.root_dir.join("src").join("unknown.arc"));
    let target_generated_by = symbol.generated_by.as_ref().or(inherited_generated_by);
    let symbol_target_path = format!("{}.{}", module.module_id, symbol.name);
    let symbol_lint_layer = lint_policy_layer_from_forewords(&symbol.forewords);
    if let Some(generated_by) = symbol.generated_by.as_ref() {
        push_generated_foreword_index_entry(
            package,
            &module.module_id,
            target_kind,
            symbol_target_path.clone(),
            public,
            generated_by,
            index,
        );
    }
    validate_foreword_apps_for_target(
        package,
        &module_path,
        &module.module_id,
        &symbol.forewords,
        target_kind,
        &symbol_target_path,
        public,
        target_generated_by,
        registry,
        inherited_lint_layers,
        warnings,
        diagnostics,
        index,
    );
    for param in &symbol.params {
        validate_foreword_apps_for_target(
            package,
            &module_path,
            &module.module_id,
            &param.forewords,
            "param",
            &format!("{}.{}({})", module.module_id, symbol.name, param.name),
            public,
            target_generated_by,
            registry,
            std::slice::from_ref(&symbol_lint_layer),
            warnings,
            diagnostics,
            index,
        );
    }
    match &symbol.body {
        HirSymbolBody::Record { fields } => {
            for field in fields {
                validate_foreword_apps_for_target(
                    package,
                    &module_path,
                    &module.module_id,
                    &field.forewords,
                    "field",
                    &format!("{}.{}.{}", module.module_id, symbol.name, field.name),
                    public,
                    target_generated_by,
                    registry,
                    std::slice::from_ref(&symbol_lint_layer),
                    warnings,
                    diagnostics,
                    index,
                );
            }
        }
        HirSymbolBody::Object { fields, methods } => {
            for field in fields {
                validate_foreword_apps_for_target(
                    package,
                    &module_path,
                    &module.module_id,
                    &field.forewords,
                    "field",
                    &format!("{}.{}.{}", module.module_id, symbol.name, field.name),
                    public,
                    target_generated_by,
                    registry,
                    std::slice::from_ref(&symbol_lint_layer),
                    warnings,
                    diagnostics,
                    index,
                );
            }
            for method in methods {
                let child_inherited_layers = std::iter::once(symbol_lint_layer.clone())
                    .chain(inherited_lint_layers.iter().cloned())
                    .collect::<Vec<_>>();
                validate_symbol_declared_forewords(
                    package,
                    module,
                    method,
                    "impl_method",
                    public,
                    target_generated_by,
                    registry,
                    &child_inherited_layers,
                    warnings,
                    diagnostics,
                    index,
                );
            }
        }
        HirSymbolBody::Trait { methods, .. } => {
            for method in methods {
                let child_inherited_layers = std::iter::once(symbol_lint_layer.clone())
                    .chain(inherited_lint_layers.iter().cloned())
                    .collect::<Vec<_>>();
                validate_symbol_declared_forewords(
                    package,
                    module,
                    method,
                    "trait_method",
                    public,
                    target_generated_by,
                    registry,
                    &child_inherited_layers,
                    warnings,
                    diagnostics,
                    index,
                );
            }
        }
        _ => {}
    }
}

fn validate_hir_semantics(
    workspace: &HirWorkspaceSummary,
    resolved: &HirResolvedWorkspace,
) -> SemanticValidation {
    let mut validation = SemanticValidation::default();
    let registry = build_foreword_registry(workspace);
    validation.errors.extend(registry.errors.clone());
    validation.foreword_catalog = registry.catalog.clone();
    for (package_id, package) in &workspace.packages {
        let Some(resolved_package) = resolved.package_by_id(package_id) else {
            continue;
        };
        validate_package_lang_item_semantics(package, &mut validation.errors);
        for module in &package.summary.modules {
            let Some(resolved_module) = resolved_package.module(&module.module_id) else {
                continue;
            };
            validate_module_foreword_semantics(
                workspace,
                package,
                module,
                resolved_module,
                &registry,
                &mut validation,
            );
            validate_module_semantics(
                workspace,
                resolved,
                package,
                module,
                resolved_module,
                &mut validation.errors,
            );
        }
    }
    validation.discovered_tests.sort_by(|left, right| {
        left.package_id
            .cmp(&right.package_id)
            .then_with(|| left.module_id.cmp(&right.module_id))
            .then_with(|| left.symbol_name.cmp(&right.symbol_name))
    });
    validation.foreword_index.sort_by(|left, right| {
        left.qualified_name
            .cmp(&right.qualified_name)
            .then_with(|| left.target_path.cmp(&right.target_path))
            .then_with(|| left.entry_kind.cmp(&right.entry_kind))
    });
    validation.foreword_registrations.sort_by(|left, right| {
        left.namespace
            .cmp(&right.namespace)
            .then_with(|| left.key.cmp(&right.key))
            .then_with(|| left.target_path.cmp(&right.target_path))
    });
    validation
}

fn validate_memory_spec_decl_semantics(
    module_path: &Path,
    spec: &arcana_hir::HirMemorySpecDecl,
    module_scope: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let family = memory_family_descriptor(spec.family);
    if spec.default_modifier.is_none() {
        diagnostics.push(Diagnostic {
            path: module_path.to_path_buf(),
            line: spec.span.line,
            column: spec.span.column,
            message: format!(
                "Memory spec `{}` requires a default modifier in v1",
                spec.name
            ),
        });
    }
    if module_scope && !family.module_specs {
        diagnostics.push(Diagnostic {
            path: module_path.to_path_buf(),
            line: spec.span.line,
            column: spec.span.column,
            message: format!(
                "memory family `{}` is not allowed at module scope",
                spec.family.as_str()
            ),
        });
    }
    if !module_scope && !family.block_specs {
        diagnostics.push(Diagnostic {
            path: module_path.to_path_buf(),
            line: spec.span.line,
            column: spec.span.column,
            message: format!(
                "memory family `{}` is not allowed in block scope",
                spec.family.as_str()
            ),
        });
    }
    if let Some(modifier) = &spec.default_modifier {
        match &modifier.kind {
            arcana_hir::HirHeadedModifierKind::Name(name)
                if memory_modifier_allowed(spec.family, name) => {}
            arcana_hir::HirHeadedModifierKind::Name(name) => diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: modifier.span.line,
                column: modifier.span.column,
                message: format!(
                    "memory modifier `-{name}` is not supported for family `{}`; allowed: {}",
                    spec.family.as_str(),
                    family.supported_modifiers.join(", ")
                ),
            }),
            arcana_hir::HirHeadedModifierKind::Keyword(keyword) => diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: modifier.span.line,
                column: modifier.span.column,
                message: format!(
                    "memory modifiers use family names, not `-{}` keyword modifiers",
                    keyword.as_str()
                ),
            }),
        }
        if modifier.payload.is_some() {
            diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: modifier.span.line,
                column: modifier.span.column,
                message: "memory modifiers do not take payload expressions in v1".to_string(),
            });
        }
    }
    let mut seen = BTreeSet::new();
    for detail in &spec.details {
        if let Some(modifier) = &detail.modifier {
            match &modifier.kind {
                arcana_hir::HirHeadedModifierKind::Name(name)
                    if memory_modifier_allowed(spec.family, name) => {}
                arcana_hir::HirHeadedModifierKind::Name(name) => diagnostics.push(Diagnostic {
                    path: module_path.to_path_buf(),
                    line: modifier.span.line,
                    column: modifier.span.column,
                    message: format!(
                        "memory modifier `-{name}` is not supported for family `{}`; allowed: {}",
                        spec.family.as_str(),
                        family.supported_modifiers.join(", ")
                    ),
                }),
                arcana_hir::HirHeadedModifierKind::Keyword(keyword) => {
                    diagnostics.push(Diagnostic {
                        path: module_path.to_path_buf(),
                        line: modifier.span.line,
                        column: modifier.span.column,
                        message: format!(
                            "memory modifiers use family names, not `-{}` keyword modifiers",
                            keyword.as_str()
                        ),
                    })
                }
            }
            if modifier.payload.is_some() {
                diagnostics.push(Diagnostic {
                    path: module_path.to_path_buf(),
                    line: modifier.span.line,
                    column: modifier.span.column,
                    message: "memory modifiers do not take payload expressions in v1".to_string(),
                });
            }
        }
        if !seen.insert(detail.key) {
            diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: detail.span.line,
                column: detail.span.column,
                message: format!(
                    "memory detail `{}` appears more than once in spec `{}`",
                    detail.key.as_str(),
                    spec.name
                ),
            });
            continue;
        }
        let Some(descriptor) = memory_detail_descriptor(spec.family, detail.key) else {
            diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: detail.span.line,
                column: detail.span.column,
                message: format!(
                    "memory detail `{}` is not supported for family `{}`",
                    detail.key.as_str(),
                    spec.family.as_str()
                ),
            });
            continue;
        };
        if descriptor.value_kind == MemoryDetailValueKind::Atom {
            let HirExpr::Path { segments } = &detail.value else {
                diagnostics.push(Diagnostic {
                    path: module_path.to_path_buf(),
                    line: detail.span.line,
                    column: detail.span.column,
                    message: format!(
                        "memory detail `{}` requires an identifier atom",
                        detail.key.as_str()
                    ),
                });
                continue;
            };
            let Some(atom) = segments.last() else {
                continue;
            };
            if !descriptor.atoms.iter().any(|allowed| *allowed == atom) {
                diagnostics.push(Diagnostic {
                    path: module_path.to_path_buf(),
                    line: detail.span.line,
                    column: detail.span.column,
                    message: format!(
                        "memory detail `{}` for family `{}` rejects atom `{}`; allowed: {}",
                        detail.key.as_str(),
                        spec.family.as_str(),
                        atom,
                        descriptor.atoms.join(", ")
                    ),
                });
            }
        }
    }
}

fn validate_recycle_modifier_semantics(
    module_path: &Path,
    scope: &ValueScope,
    modifier: &arcana_hir::HirHeadedModifier,
    gate_shape: Option<&GateShape>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match &modifier.kind {
        arcana_hir::HirHeadedModifierKind::Keyword(HeadedModifierKeyword::Break)
            if scope.loop_depth == 0 =>
        {
            diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: modifier.span.line,
                column: modifier.span.column,
                message: "`recycle -break` is only valid inside a loop".to_string(),
            });
        }
        arcana_hir::HirHeadedModifierKind::Keyword(HeadedModifierKeyword::Continue)
            if scope.loop_depth == 0 =>
        {
            diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: modifier.span.line,
                column: modifier.span.column,
                message: "`recycle -continue` is only valid inside a loop".to_string(),
            });
        }
        arcana_hir::HirHeadedModifierKind::Keyword(HeadedModifierKeyword::Return) => {
            if modifier.payload.is_none() && !matches!(gate_shape, Some(GateShape::Result { .. })) {
                diagnostics.push(Diagnostic {
                    path: module_path.to_path_buf(),
                    line: modifier.span.line,
                    column: modifier.span.column,
                    message: "bare `-return` in recycle requires Result failure".to_string(),
                });
            }
        }
        arcana_hir::HirHeadedModifierKind::Name(name) => match scope.active_owner_for_exit(name) {
            Ok(Some(_)) => {}
            Ok(None) => diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: modifier.span.line,
                column: modifier.span.column,
                message: format!("named recycle exit `{name}` is not active on this path"),
            }),
            Err(message) => diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: modifier.span.line,
                column: modifier.span.column,
                message,
            }),
        },
        _ => {}
    }
}

fn validate_bind_modifier_semantics(
    module_path: &Path,
    scope: &ValueScope,
    modifier: &arcana_hir::HirHeadedModifier,
    gate_shape: Option<&GateShape>,
    line_kind: &arcana_hir::HirBindLineKind,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match &modifier.kind {
        arcana_hir::HirHeadedModifierKind::Keyword(HeadedModifierKeyword::Return) => {
            if modifier.payload.is_none() && !matches!(gate_shape, Some(GateShape::Result { .. })) {
                diagnostics.push(Diagnostic {
                    path: module_path.to_path_buf(),
                    line: modifier.span.line,
                    column: modifier.span.column,
                    message: "bare `-return` in bind requires Result failure".to_string(),
                });
            }
        }
        arcana_hir::HirHeadedModifierKind::Keyword(HeadedModifierKeyword::Default) => {
            if !matches!(line_kind, arcana_hir::HirBindLineKind::Let { .. }) {
                diagnostics.push(Diagnostic {
                    path: module_path.to_path_buf(),
                    line: modifier.span.line,
                    column: modifier.span.column,
                    message: "`bind -default` is only valid on `let name = gate` lines".to_string(),
                });
            }
            if modifier.payload.is_none() {
                diagnostics.push(Diagnostic {
                    path: module_path.to_path_buf(),
                    line: modifier.span.line,
                    column: modifier.span.column,
                    message: "`bind -default` requires a fallback payload".to_string(),
                });
            }
        }
        arcana_hir::HirHeadedModifierKind::Keyword(HeadedModifierKeyword::Preserve) => {
            if !matches!(line_kind, arcana_hir::HirBindLineKind::Assign { .. }) {
                diagnostics.push(Diagnostic {
                    path: module_path.to_path_buf(),
                    line: modifier.span.line,
                    column: modifier.span.column,
                    message: "`bind -preserve` is only valid on `name = gate` lines".to_string(),
                });
            }
        }
        arcana_hir::HirHeadedModifierKind::Keyword(HeadedModifierKeyword::Replace) => {
            if !matches!(line_kind, arcana_hir::HirBindLineKind::Assign { .. }) {
                diagnostics.push(Diagnostic {
                    path: module_path.to_path_buf(),
                    line: modifier.span.line,
                    column: modifier.span.column,
                    message: "`bind -replace` is only valid on `name = gate` lines".to_string(),
                });
            }
            if modifier.payload.is_none() {
                diagnostics.push(Diagnostic {
                    path: module_path.to_path_buf(),
                    line: modifier.span.line,
                    column: modifier.span.column,
                    message: "`bind -replace` requires a fallback payload".to_string(),
                });
            }
        }
        arcana_hir::HirHeadedModifierKind::Keyword(HeadedModifierKeyword::Break)
            if !matches!(line_kind, arcana_hir::HirBindLineKind::Require { .. }) =>
        {
            diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: modifier.span.line,
                column: modifier.span.column,
                message: "`bind -break` is only valid on `require <expr>` lines".to_string(),
            });
        }
        arcana_hir::HirHeadedModifierKind::Keyword(HeadedModifierKeyword::Continue)
            if !matches!(line_kind, arcana_hir::HirBindLineKind::Require { .. }) =>
        {
            diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: modifier.span.line,
                column: modifier.span.column,
                message: "`bind -continue` is only valid on `require <expr>` lines".to_string(),
            });
        }
        arcana_hir::HirHeadedModifierKind::Keyword(HeadedModifierKeyword::Break)
            if scope.loop_depth == 0 =>
        {
            diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: modifier.span.line,
                column: modifier.span.column,
                message: "`bind -break` is only valid inside a loop".to_string(),
            });
        }
        arcana_hir::HirHeadedModifierKind::Keyword(HeadedModifierKeyword::Continue)
            if scope.loop_depth == 0 =>
        {
            diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: modifier.span.line,
                column: modifier.span.column,
                message: "`bind -continue` is only valid inside a loop".to_string(),
            });
        }
        _ => {}
    }
    if matches!(line_kind, arcana_hir::HirBindLineKind::Require { .. })
        && !matches!(
            modifier.kind,
            arcana_hir::HirHeadedModifierKind::Keyword(
                HeadedModifierKeyword::Return
                    | HeadedModifierKeyword::Break
                    | HeadedModifierKeyword::Continue
            )
        )
    {
        diagnostics.push(Diagnostic {
            path: module_path.to_path_buf(),
            line: modifier.span.line,
            column: modifier.span.column,
            message:
                "`bind require` only supports `return`, `break`, or `continue` failure handling"
                    .to_string(),
        });
    }
}

fn validate_construct_modifier_semantics(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    type_scope: &TypeScope,
    scope: &ValueScope,
    modifier: &arcana_hir::HirHeadedModifier,
    value: &HirExpr,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match &modifier.kind {
        arcana_hir::HirHeadedModifierKind::Keyword(HeadedModifierKeyword::Return) => {
            if modifier.payload.is_none()
                && !matches!(
                    infer_gate_shape(workspace, resolved_module, type_scope, scope, value),
                    Some(GateShape::Result { .. })
                )
            {
                diagnostics.push(Diagnostic {
                    path: module_path.to_path_buf(),
                    line: modifier.span.line,
                    column: modifier.span.column,
                    message: "bare `-return` on construct requires Result failure".to_string(),
                });
            }
        }
        arcana_hir::HirHeadedModifierKind::Keyword(HeadedModifierKeyword::Default) => {
            if modifier.payload.is_none() {
                diagnostics.push(Diagnostic {
                    path: module_path.to_path_buf(),
                    line: modifier.span.line,
                    column: modifier.span.column,
                    message: "`construct -default` requires a fallback payload".to_string(),
                });
            }
        }
        arcana_hir::HirHeadedModifierKind::Keyword(HeadedModifierKeyword::Skip) => {
            if modifier.payload.is_some() {
                diagnostics.push(Diagnostic {
                    path: module_path.to_path_buf(),
                    line: modifier.span.line,
                    column: modifier.span.column,
                    message: "`construct -skip` does not take a payload".to_string(),
                });
            }
        }
        arcana_hir::HirHeadedModifierKind::Keyword(other) => {
            diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: modifier.span.line,
                column: modifier.span.column,
                message: format!(
                    "`construct` does not support `-{}` modifiers in v1",
                    other.as_str()
                ),
            });
        }
        arcana_hir::HirHeadedModifierKind::Name(name) => {
            diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: modifier.span.line,
                column: modifier.span.column,
                message: format!("`construct` does not support named modifier `-{name}` in v1"),
            });
        }
    }
}

fn validate_construct_contribution_semantics(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    type_scope: &TypeScope,
    scope: &ValueScope,
    expected_return_type: Option<&HirType>,
    line: &arcana_hir::HirConstructLine,
    target_name: &str,
    target_ty: &HirType,
    modifier: Option<&arcana_hir::HirHeadedModifier>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(mode) = infer_construct_contribution_mode(
        workspace,
        resolved_module,
        type_scope,
        scope,
        &line.value,
        target_ty,
    ) else {
        let expected = canonicalize_local_hir_type(workspace, resolved_module, target_ty)
            .unwrap_or_else(|| target_ty.clone());
        let actual =
            infer_expr_value_type(workspace, resolved_module, type_scope, scope, &line.value)
                .and_then(|ty| canonicalize_local_hir_type(workspace, resolved_module, &ty))
                .map(|ty| ty.render())
                .unwrap_or_else(|| "unknown".to_string());
        diagnostics.push(Diagnostic {
            path: module_path.to_path_buf(),
            line: line.span.line,
            column: line.span.column,
            message: format!(
                "construct contribution `{}` has type `{actual}` but target `{target_name}` expects `{}` or sanctioned Option/Result acquisition into that type",
                line.name,
                expected.render()
            ),
        });
        return;
    };
    if let Some(modifier) = modifier {
        validate_construct_modifier_semantics(
            workspace,
            resolved_module,
            module_path,
            type_scope,
            scope,
            modifier,
            &line.value,
            diagnostics,
        );
        if matches!(
            modifier.kind,
            arcana_hir::HirHeadedModifierKind::Keyword(HeadedModifierKeyword::Default)
        ) && let Some(payload) = &modifier.payload
        {
            let expected = canonicalize_local_hir_type(workspace, resolved_module, target_ty)
                .unwrap_or_else(|| target_ty.clone());
            let expected_key = expected.render();
            let actual =
                infer_expr_value_type(workspace, resolved_module, type_scope, scope, payload)
                    .and_then(|ty| canonicalize_local_hir_type(workspace, resolved_module, &ty));
            if actual.as_ref().map(|ty| ty.render()).as_deref() != Some(expected_key.as_str()) {
                diagnostics.push(Diagnostic {
                    path: module_path.to_path_buf(),
                    line: modifier.span.line,
                    column: modifier.span.column,
                    message: format!(
                        "`construct -default` fallback for `{}` must have type `{}`",
                        line.name,
                        expected.render()
                    ),
                });
            }
        }
        if matches!(
            modifier.kind,
            arcana_hir::HirHeadedModifierKind::Keyword(HeadedModifierKeyword::Return)
        ) && modifier.payload.is_none()
            && !matches!(mode, ConstructContributionMode::ResultPayload)
            && matches!(
                infer_gate_shape(workspace, resolved_module, type_scope, scope, &line.value),
                Some(GateShape::Result { .. })
            )
            && infer_construct_contribution_mode(
                workspace,
                resolved_module,
                type_scope,
                scope,
                &line.value,
                target_ty,
            ) == Some(ConstructContributionMode::Direct)
        {
            diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: modifier.span.line,
                column: modifier.span.column,
                message: "bare `-return` on construct only applies to Result payload acquisition, not direct Result values".to_string(),
            });
        }
        if matches!(
            modifier.kind,
            arcana_hir::HirHeadedModifierKind::Keyword(HeadedModifierKeyword::Return)
        ) && modifier.payload.is_none()
            && matches!(mode, ConstructContributionMode::ResultPayload)
        {
            validate_bare_result_return_type(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                &line.value,
                expected_return_type,
                modifier.span,
                "bare `construct -return` failure",
                diagnostics,
            );
        }
    }
}

fn validate_construct_region_semantics(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    type_scope: &TypeScope,
    scope: &ValueScope,
    expected_return_type: Option<&HirType>,
    region: &arcana_hir::HirConstructRegion,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if region.default_modifier.is_none() {
        diagnostics.push(Diagnostic {
            path: module_path.to_path_buf(),
            line: region.span.line,
            column: region.span.column,
            message: "construct requires a default modifier in v1".to_string(),
        });
    }
    let Some(target_path) = flatten_callable_expr_path(&region.target) else {
        diagnostics.push(Diagnostic {
            path: module_path.to_path_buf(),
            line: region.span.line,
            column: region.span.column,
            message: "construct target must be a path-like constructor reference".to_string(),
        });
        return;
    };
    let Some(target_shape) =
        resolve_construct_target_shape(workspace, resolved_module, &target_path)
    else {
        diagnostics.push(Diagnostic {
            path: module_path.to_path_buf(),
            line: region.span.line,
            column: region.span.column,
            message: format!(
                "construct target `{}` must resolve to a record or single-payload enum variant",
                target_path.join(".")
            ),
        });
        return;
    };
    if let Some(arcana_hir::HirConstructDestination::Place { target }) = &region.destination {
        let expected_ty = resolve_construct_result_type(workspace, resolved_module, &target_path);
        let actual_ty =
            infer_assign_target_value_type(workspace, resolved_module, type_scope, scope, target)
                .and_then(|ty| canonicalize_local_hir_type(workspace, resolved_module, &ty));
        match (expected_ty, actual_ty) {
            (Some(expected_ty), Some(actual_ty))
                if canonical_hir_type_key(workspace, resolved_module, &expected_ty)
                    != canonical_hir_type_key(workspace, resolved_module, &actual_ty) =>
            {
                diagnostics.push(Diagnostic {
                    path: module_path.to_path_buf(),
                    line: region.span.line,
                    column: region.span.column,
                    message: format!(
                        "construct place target type `{}` does not match constructor result type `{}`",
                        actual_ty.render(),
                        expected_ty.render()
                    ),
                });
            }
            (Some(_), None) => {
                diagnostics.push(Diagnostic {
                    path: module_path.to_path_buf(),
                    line: region.span.line,
                    column: region.span.column,
                    message: "construct place target must have a known type in v1".to_string(),
                });
            }
            _ => {}
        }
    }
    if let Some(arcana_hir::HirConstructDestination::Deliver { name }) = &region.destination
        && scope.contains(name)
    {
        diagnostics.push(Diagnostic {
            path: module_path.to_path_buf(),
            line: region.span.line,
            column: region.span.column,
            message: format!("construct deliver binding `{name}` already exists in this scope"),
        });
    }

    let mut seen = BTreeSet::new();
    match &target_shape {
        ConstructTargetShape::Record { fields } => {
            for line in &region.lines {
                if !seen.insert(line.name.clone()) {
                    diagnostics.push(Diagnostic {
                        path: module_path.to_path_buf(),
                        line: line.span.line,
                        column: line.span.column,
                        message: format!(
                            "construct field `{}` is provided more than once",
                            line.name
                        ),
                    });
                }
                let Some(field_ty) = fields.get(&line.name) else {
                    diagnostics.push(Diagnostic {
                        path: module_path.to_path_buf(),
                        line: line.span.line,
                        column: line.span.column,
                        message: format!(
                            "construct field `{}` does not exist on `{}`",
                            line.name,
                            target_path.join(".")
                        ),
                    });
                    continue;
                };
                let modifier = line.modifier.as_ref().or(region.default_modifier.as_ref());
                validate_construct_contribution_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    expected_return_type,
                    line,
                    &format!("{}.{}", target_path.join("."), line.name),
                    field_ty,
                    modifier,
                    diagnostics,
                );
                if let Some(modifier) = modifier
                    && matches!(
                        modifier.kind,
                        arcana_hir::HirHeadedModifierKind::Keyword(HeadedModifierKeyword::Skip)
                    )
                    && type_option_payload(field_ty).is_none()
                {
                    diagnostics.push(Diagnostic {
                        path: module_path.to_path_buf(),
                        line: modifier.span.line,
                        column: modifier.span.column,
                        message: format!(
                            "construct `-skip` is only valid for Option fields; `{}` is `{}`",
                            line.name,
                            field_ty.render()
                        ),
                    });
                }
            }
            for (field_name, field_ty) in fields {
                if !seen.contains(field_name) && type_option_payload(field_ty).is_none() {
                    diagnostics.push(Diagnostic {
                        path: module_path.to_path_buf(),
                        line: region.span.line,
                        column: region.span.column,
                        message: format!(
                            "construct target `{}` is missing required field `{field_name}`",
                            target_path.join(".")
                        ),
                    });
                }
            }
        }
        ConstructTargetShape::Variant { payload } => {
            let payload_lines = region
                .lines
                .iter()
                .filter(|line| line.name == "payload")
                .count();
            if payload_lines != 1 {
                diagnostics.push(Diagnostic {
                    path: module_path.to_path_buf(),
                    line: region.span.line,
                    column: region.span.column,
                    message: format!(
                        "construct target `{}` requires exactly one `payload = ...` line",
                        target_path.join(".")
                    ),
                });
            }
            for line in &region.lines {
                if line.name != "payload" {
                    diagnostics.push(Diagnostic {
                        path: module_path.to_path_buf(),
                        line: line.span.line,
                        column: line.span.column,
                        message: format!(
                            "construct target `{}` only accepts `payload = ...` contributions",
                            target_path.join(".")
                        ),
                    });
                }
                let modifier = line.modifier.as_ref().or(region.default_modifier.as_ref());
                validate_construct_contribution_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    expected_return_type,
                    line,
                    &format!("{}.payload", target_path.join(".")),
                    payload,
                    modifier,
                    diagnostics,
                );
                if let Some(modifier) = modifier
                    && matches!(
                        modifier.kind,
                        arcana_hir::HirHeadedModifierKind::Keyword(HeadedModifierKeyword::Skip)
                    )
                    && type_option_payload(payload).is_none()
                {
                    diagnostics.push(Diagnostic {
                        path: module_path.to_path_buf(),
                        line: modifier.span.line,
                        column: modifier.span.column,
                        message: format!(
                            "construct `-skip` is only valid for Option payloads; payload is `{}`",
                            payload.render()
                        ),
                    });
                }
            }
        }
    }
}

fn validate_record_region_semantics(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    type_scope: &TypeScope,
    scope: &ValueScope,
    expected_return_type: Option<&HirType>,
    region: &arcana_hir::HirRecordRegion,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if region.default_modifier.is_none() {
        diagnostics.push(Diagnostic {
            path: module_path.to_path_buf(),
            line: region.span.line,
            column: region.span.column,
            message: "record requires a default modifier in v1".to_string(),
        });
    }
    let Some(target_path) = flatten_callable_expr_path(&region.target) else {
        diagnostics.push(Diagnostic {
            path: module_path.to_path_buf(),
            line: region.span.line,
            column: region.span.column,
            message: "record target must be a path-like record reference".to_string(),
        });
        return;
    };
    let Some(fields) = resolve_record_target_fields(workspace, resolved_module, &target_path) else {
        diagnostics.push(Diagnostic {
            path: module_path.to_path_buf(),
            line: region.span.line,
            column: region.span.column,
            message: format!(
                "record target `{}` must resolve to a record",
                target_path.join(".")
            ),
        });
        return;
    };
    if let Some(arcana_hir::HirConstructDestination::Place { target }) = &region.destination {
        let expected_ty = resolve_record_result_type(workspace, resolved_module, &target_path);
        let actual_ty =
            infer_assign_target_value_type(workspace, resolved_module, type_scope, scope, target)
                .and_then(|ty| canonicalize_local_hir_type(workspace, resolved_module, &ty));
        match (expected_ty, actual_ty) {
            (Some(expected_ty), Some(actual_ty))
                if canonical_hir_type_key(workspace, resolved_module, &expected_ty)
                    != canonical_hir_type_key(workspace, resolved_module, &actual_ty) =>
            {
                diagnostics.push(Diagnostic {
                    path: module_path.to_path_buf(),
                    line: region.span.line,
                    column: region.span.column,
                    message: format!(
                        "record place target type `{}` does not match record result type `{}`",
                        actual_ty.render(),
                        expected_ty.render()
                    ),
                });
            }
            (Some(_), None) => {
                diagnostics.push(Diagnostic {
                    path: module_path.to_path_buf(),
                    line: region.span.line,
                    column: region.span.column,
                    message: "record place target must have a known type in v1".to_string(),
                });
            }
            _ => {}
        }
    }
    if let Some(arcana_hir::HirConstructDestination::Deliver { name }) = &region.destination
        && scope.contains(name)
    {
        diagnostics.push(Diagnostic {
            path: module_path.to_path_buf(),
            line: region.span.line,
            column: region.span.column,
            message: format!("record deliver binding `{name}` already exists in this scope"),
        });
    }

    let base_fields = region
        .base
        .as_ref()
        .and_then(|base| infer_expr_value_type(workspace, resolved_module, type_scope, scope, base))
        .and_then(|ty| canonicalize_local_hir_type(workspace, resolved_module, &ty))
        .and_then(|ty| resolve_record_fields_for_type(workspace, resolved_module, &ty));

    if region.base.is_some() && base_fields.is_none() {
        diagnostics.push(Diagnostic {
            path: module_path.to_path_buf(),
            line: region.span.line,
            column: region.span.column,
            message: "record base must have a known record type in v1".to_string(),
        });
    }

    let mut seen = BTreeSet::new();
    for line in &region.lines {
        if !seen.insert(line.name.clone()) {
            diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: line.span.line,
                column: line.span.column,
                message: format!("record field `{}` is provided more than once", line.name),
            });
        }
        let Some(field_ty) = fields.get(&line.name) else {
            diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: line.span.line,
                column: line.span.column,
                message: format!(
                    "record field `{}` does not exist on `{}`",
                    line.name,
                    target_path.join(".")
                ),
            });
            continue;
        };
        let modifier = line.modifier.as_ref().or(region.default_modifier.as_ref());
        validate_construct_contribution_semantics(
            workspace,
            resolved_module,
            module_path,
            type_scope,
            scope,
            expected_return_type,
            line,
            &format!("{}.{}", target_path.join("."), line.name),
            field_ty,
            modifier,
            diagnostics,
        );
        if let Some(modifier) = modifier
            && matches!(
                modifier.kind,
                arcana_hir::HirHeadedModifierKind::Keyword(HeadedModifierKeyword::Skip)
            )
            && type_option_payload(field_ty).is_none()
        {
            diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: modifier.span.line,
                column: modifier.span.column,
                message: format!(
                    "record `-skip` is only valid for Option fields; `{}` is `{}`",
                    line.name,
                    field_ty.render()
                ),
            });
        }
    }

    for (field_name, field_ty) in &fields {
        if seen.contains(field_name) {
            continue;
        }
        let exact_base_match = base_fields.as_ref().and_then(|base_fields| {
            base_fields.get(field_name).filter(|base_ty| {
                canonical_hir_type_key(workspace, resolved_module, base_ty)
                    == canonical_hir_type_key(workspace, resolved_module, field_ty)
            })
        });
        if exact_base_match.is_some() {
            continue;
        }
        if let Some(base_ty) = base_fields.as_ref().and_then(|base_fields| base_fields.get(field_name))
            && canonical_hir_type_key(workspace, resolved_module, base_ty)
                != canonical_hir_type_key(workspace, resolved_module, field_ty)
            && type_option_payload(field_ty).is_none()
        {
            diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: region.span.line,
                column: region.span.column,
                message: format!(
                    "record base field `{field_name}` has incompatible type `{}` for target `{}` field `{}`",
                    base_ty.render(),
                    target_path.join("."),
                    field_ty.render()
                ),
            });
            continue;
        }
        if type_option_payload(field_ty).is_none() {
            diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: region.span.line,
                column: region.span.column,
                message: format!(
                    "record target `{}` is missing required field `{field_name}`",
                    target_path.join(".")
                ),
            });
        }
    }
}

fn validate_module_foreword_semantics(
    workspace: &HirWorkspaceSummary,
    package: &HirWorkspacePackage,
    module: &HirModuleSummary,
    resolved_module: &HirResolvedModule,
    registry: &ForewordRegistry,
    validation: &mut SemanticValidation,
) {
    let module_path = package
        .module_path(&module.module_id)
        .cloned()
        .unwrap_or_else(|| package.root_dir.join("src").join("unknown.arc"));

    for directive in &module.directives {
        let target_kind = match directive.kind {
            arcana_hir::HirDirectiveKind::Import => "import",
            arcana_hir::HirDirectiveKind::Use => "use",
            arcana_hir::HirDirectiveKind::Reexport => "reexport",
        };
        validate_foreword_apps_for_target(
            package,
            &module_path,
            &module.module_id,
            &directive.forewords,
            target_kind,
            &format!("{}:{}", module.module_id, directive.path.join(".")),
            directive.kind == arcana_hir::HirDirectiveKind::Reexport,
            None,
            registry,
            &[],
            &mut validation.warnings,
            &mut validation.errors,
            &mut validation.foreword_index,
        );
    }
    for entry in &module.emitted_foreword_metadata {
        push_emitted_foreword_index_entry(
            package,
            &module.module_id,
            entry,
            &mut validation.foreword_index,
        );
    }
    validation.foreword_registrations.extend(
        module
            .foreword_registrations
            .iter()
            .map(lower_registration_row),
    );

    for symbol in &module.symbols {
        validate_symbol_declared_forewords(
            package,
            module,
            symbol,
            symbol.kind.as_str(),
            symbol.exported,
            None,
            registry,
            &[],
            &mut validation.warnings,
            &mut validation.errors,
            &mut validation.foreword_index,
        );
        if symbol.kind == HirSymbolKind::Fn
            && symbol
                .forewords
                .iter()
                .any(|foreword| foreword.path.len() == 1 && foreword.name == "test")
        {
            validation.discovered_tests.push(DiscoveredTest {
                package_id: package.package_id.clone(),
                module_id: module.module_id.clone(),
                symbol_name: symbol.name.clone(),
            });
        }
        let mut scope = ValueScope::default().with_symbol_params(&symbol.params);
        let policy =
            lint_policy_from_layers(vec![lint_policy_layer_from_forewords(&symbol.forewords)]);
        collect_deprecated_call_warnings_in_statements(
            workspace,
            resolved_module,
            &module_path,
            &TypeScope::default().with_params(&symbol.type_params),
            &mut scope,
            &symbol.statements,
            &policy,
            &mut validation.warnings,
            &mut validation.errors,
        );
        match &symbol.body {
            HirSymbolBody::Object { methods, .. } | HirSymbolBody::Trait { methods, .. } => {
                for method in methods {
                    let mut method_scope = ValueScope::default().with_symbol_params(&method.params);
                    let method_policy = lint_policy_from_layers(vec![
                        lint_policy_layer_from_forewords(&method.forewords),
                        lint_policy_layer_from_forewords(&symbol.forewords),
                    ]);
                    collect_deprecated_call_warnings_in_statements(
                        workspace,
                        resolved_module,
                        &module_path,
                        &TypeScope::default().with_params(&method.type_params),
                        &mut method_scope,
                        &method.statements,
                        &method_policy,
                        &mut validation.warnings,
                        &mut validation.errors,
                    );
                }
            }
            _ => {}
        }
    }

    for impl_decl in &module.impls {
        if let Some(generated_by) = impl_decl.generated_by.as_ref() {
            push_generated_foreword_index_entry(
                package,
                &module.module_id,
                "impl",
                impl_target_path(&module.module_id, impl_decl),
                false,
                generated_by,
                &mut validation.foreword_index,
            );
        }
        for method in &impl_decl.methods {
            validate_symbol_declared_forewords(
                package,
                module,
                method,
                "impl_method",
                false,
                impl_decl.generated_by.as_ref(),
                registry,
                &[],
                &mut validation.warnings,
                &mut validation.errors,
                &mut validation.foreword_index,
            );
            let mut scope = ValueScope::default().with_symbol_params(&method.params);
            let policy =
                lint_policy_from_layers(vec![lint_policy_layer_from_forewords(&method.forewords)]);
            collect_deprecated_call_warnings_in_statements(
                workspace,
                resolved_module,
                &module_path,
                &TypeScope::default().with_params(&method.type_params),
                &mut scope,
                &method.statements,
                &policy,
                &mut validation.warnings,
                &mut validation.errors,
            );
        }
    }
}

fn validate_module_semantics(
    workspace: &HirWorkspaceSummary,
    resolved_workspace: &HirResolvedWorkspace,
    package: &HirWorkspacePackage,
    module: &HirModuleSummary,
    resolved_module: &HirResolvedModule,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let module_path = package
        .module_path(&module.module_id)
        .cloned()
        .unwrap_or_else(|| package.root_dir.join("src").join("unknown.arc"));

    for lang_item in &module.lang_items {
        let Some(symbol_ref) =
            lookup_symbol_path(workspace, resolved_module, lang_item.target.as_slice())
        else {
            diagnostics.push(Diagnostic {
                path: module_path.clone(),
                line: lang_item.span.line,
                column: lang_item.span.column,
                message: format!(
                    "unresolved `lang` item target `{}` for `{}`",
                    lang_item.target.join("."),
                    lang_item.name
                ),
            });
            continue;
        };
        if let Some(family) = opaque_lang_family(&lang_item.name) {
            if symbol_ref.symbol.kind != HirSymbolKind::OpaqueType {
                diagnostics.push(Diagnostic {
                    path: module_path.clone(),
                    line: lang_item.span.line,
                    column: lang_item.span.column,
                    message: format!(
                        "opaque family lang item `{}` must target an opaque type, found `{}`",
                        family.name(),
                        symbol_ref.symbol.kind.as_str()
                    ),
                });
                continue;
            }
            let actual_ownership = ownership_of_opaque_symbol(symbol_ref.symbol);
            let expected_ownership = family.expected_ownership();
            if actual_ownership != expected_ownership {
                diagnostics.push(Diagnostic {
                    path: module_path.clone(),
                    line: lang_item.span.line,
                    column: lang_item.span.column,
                    message: format!(
                        "opaque family lang item `{}` must target a {} opaque type",
                        family.name(),
                        match expected_ownership {
                            OwnershipClass::Copy => "copy",
                            OwnershipClass::Move => "move",
                            OwnershipClass::Unknown => "known-ownership",
                        }
                    ),
                });
            }
            if !opaque_symbol_is_boundary_unsafe(symbol_ref.symbol) {
                diagnostics.push(Diagnostic {
                    path: module_path.clone(),
                    line: lang_item.span.line,
                    column: lang_item.span.column,
                    message: format!(
                        "opaque family lang item `{}` must target a boundary_unsafe opaque type",
                        family.name()
                    ),
                });
            }
        } else if lang_item.name == "cleanup_contract"
            && symbol_ref.symbol.kind != HirSymbolKind::Trait
        {
            diagnostics.push(Diagnostic {
                path: module_path.clone(),
                line: lang_item.span.line,
                column: lang_item.span.column,
                message: format!(
                    "cleanup contract lang item `cleanup_contract` must target a trait, found `{}`",
                    symbol_ref.symbol.kind.as_str()
                ),
            });
        }
    }

    let mut seen_memory_specs = BTreeSet::new();
    for spec in &module.memory_specs {
        if !seen_memory_specs.insert(spec.name.clone()) {
            diagnostics.push(Diagnostic {
                path: module_path.clone(),
                line: spec.span.line,
                column: spec.span.column,
                message: format!(
                    "module memory spec `{}` is declared more than once",
                    spec.name
                ),
            });
        }
        validate_memory_spec_decl_semantics(&module_path, spec, true, diagnostics);
        let region_scope = ValueScope {
            headed_region_depth: 1,
            ..ValueScope::default()
        };
        if let Some(modifier) = &spec.default_modifier
            && let Some(payload) = &modifier.payload
        {
            validate_expr_semantics(
                workspace,
                resolved_module,
                &module_path,
                &TypeScope::default(),
                &region_scope,
                payload,
                spec.span,
                diagnostics,
            );
        }
        for detail in &spec.details {
            if memory_detail_descriptor(spec.family, detail.key)
                .map(|descriptor| descriptor.value_kind == MemoryDetailValueKind::IntExpr)
                .unwrap_or(true)
            {
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    &module_path,
                    &TypeScope::default(),
                    &region_scope,
                    &detail.value,
                    detail.span,
                    diagnostics,
                );
            }
            if let Some(modifier) = &detail.modifier
                && let Some(payload) = &modifier.payload
            {
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    &module_path,
                    &TypeScope::default(),
                    &region_scope,
                    payload,
                    detail.span,
                    diagnostics,
                );
            }
        }
    }

    for symbol in &module.symbols {
        if is_runtime_main_entry_symbol(&package.summary.package_name, &module.module_id, symbol)
            && let Err(message) = validate_runtime_main_entry_symbol(symbol)
        {
            diagnostics.push(Diagnostic {
                path: module_path.clone(),
                line: symbol.span.line,
                column: symbol.span.column,
                message,
            });
        }
        if symbol.kind == HirSymbolKind::OpaqueType && is_builtin_type_name(&symbol.name) {
            diagnostics.push(Diagnostic {
                path: module_path.clone(),
                line: symbol.span.line,
                column: symbol.span.column,
                message: format!(
                    "opaque type `{}` conflicts with reserved builtin type name",
                    symbol.name
                ),
            });
        }
        validate_symbol_surface_types(
            workspace,
            resolved_workspace,
            resolved_module,
            &module_path,
            symbol,
            &TypeScope::default(),
            diagnostics,
        );
        let symbol_scope = TypeScope::default().with_params(&symbol.type_params);
        validate_boundary_symbol_contract(
            workspace,
            resolved_workspace,
            resolved_module,
            &module_path,
            symbol,
            &symbol_scope,
            None,
            &BTreeMap::new(),
            diagnostics,
        );
        validate_symbol_value_semantics(
            workspace,
            resolved_workspace,
            resolved_module,
            &module_path,
            symbol,
            &TypeScope::default(),
            &ValueScope::default(),
            diagnostics,
        );
    }
    for impl_decl in &module.impls {
        validate_impl_surface_types(
            workspace,
            resolved_workspace,
            resolved_module,
            &module_path,
            impl_decl,
            diagnostics,
        );
        validate_impl_value_semantics(
            workspace,
            resolved_workspace,
            resolved_module,
            &module_path,
            impl_decl,
            diagnostics,
        );
    }
}

fn validate_symbol_surface_types(
    workspace: &HirWorkspaceSummary,
    resolved_workspace: &HirResolvedWorkspace,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    symbol: &HirSymbol,
    inherited_scope: &TypeScope,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let scope = inherited_scope.with_params(&symbol.type_params);
    if symbol.kind == HirSymbolKind::OpaqueType && symbol.opaque_policy.is_none() {
        diagnostics.push(Diagnostic {
            path: module_path.to_path_buf(),
            line: symbol.span.line,
            column: symbol.span.column,
            message: format!(
                "opaque type `{}` is missing required policy atoms",
                symbol.name
            ),
        });
    }
    for param in &symbol.params {
        validate_type_surface(
            workspace,
            resolved_module,
            module_path,
            &scope,
            &param.ty,
            symbol.span,
            &format!("parameter type `{}`", param.name),
            diagnostics,
        );
    }
    if let Some(return_type) = &symbol.return_type {
        validate_type_surface(
            workspace,
            resolved_module,
            module_path,
            &scope,
            return_type,
            symbol.span,
            "return type",
            diagnostics,
        );
        for lifetime in collect_hir_type_refs(return_type).lifetimes {
            if lifetime != "'static" && symbol.params.is_empty() {
                diagnostics.push(Diagnostic {
                    path: module_path.to_path_buf(),
                    line: symbol.span.line,
                    column: symbol.span.column,
                    message: format!(
                        "return lifetime `{lifetime}` must be tied to an input parameter"
                    ),
                });
            }
        }
    }
    if let Some(where_clause) = &symbol.where_clause {
        validate_where_clause_surface(
            workspace,
            resolved_module,
            module_path,
            &scope,
            where_clause,
            symbol.span,
            diagnostics,
        );
    }
    match &symbol.body {
        HirSymbolBody::None => {}
        HirSymbolBody::Record { fields } => {
            for field in fields {
                validate_type_surface(
                    workspace,
                    resolved_module,
                    module_path,
                    &scope,
                    &field.ty,
                    field.span,
                    &format!("field type `{}`", field.name),
                    diagnostics,
                );
            }
        }
        HirSymbolBody::Object { fields, methods } => {
            let object_scope = scope.with_self();
            for field in fields {
                validate_type_surface(
                    workspace,
                    resolved_module,
                    module_path,
                    &object_scope,
                    &field.ty,
                    field.span,
                    &format!("object field type `{}`", field.name),
                    diagnostics,
                );
            }
            for method in methods {
                validate_symbol_surface_types(
                    workspace,
                    resolved_workspace,
                    resolved_module,
                    module_path,
                    method,
                    &object_scope,
                    diagnostics,
                );
            }
            let _ = collect_object_lifecycle_surface(
                workspace,
                resolved_module,
                module_path,
                symbol,
                diagnostics,
            );
        }
        HirSymbolBody::Enum { variants } => {
            for variant in variants {
                if let Some(payload) = &variant.payload {
                    validate_type_surface(
                        workspace,
                        resolved_module,
                        module_path,
                        &scope,
                        payload,
                        variant.span,
                        &format!("enum variant payload `{}`", variant.name),
                        diagnostics,
                    );
                }
            }
        }
        HirSymbolBody::Owner {
            objects,
            context_type,
            exits,
        } => {
            if exits.is_empty() {
                diagnostics.push(Diagnostic {
                    path: module_path.to_path_buf(),
                    line: symbol.span.line,
                    column: symbol.span.column,
                    message: format!(
                        "owner `{}` must declare at least one scope-exit",
                        symbol.name
                    ),
                });
            }
            let mut seen_owned_names = BTreeSet::new();
            for object in objects {
                if !seen_owned_names.insert(object.local_name.clone()) {
                    diagnostics.push(Diagnostic {
                        path: module_path.to_path_buf(),
                        line: object.span.line,
                        column: object.span.column,
                        message: format!(
                            "owner object `{}` is declared more than once",
                            object.local_name
                        ),
                    });
                }
                validate_surface_path_kind(
                    workspace,
                    resolved_module,
                    module_path,
                    &scope,
                    &object.type_path,
                    object.span,
                    &format!("owner object type `{}`", object.local_name),
                    SurfaceSymbolUse::TypeLike,
                    diagnostics,
                );
                if let Some(resolved_object) =
                    lookup_symbol_path(workspace, resolved_module, &object.type_path)
                {
                    let object_resolved_module = resolved_workspace
                        .package(resolved_object.package_name)
                        .and_then(|package| package.module(resolved_object.module_id))
                        .unwrap_or(resolved_module);
                    let object_module_path = workspace
                        .package(resolved_object.package_name)
                        .and_then(|package| package.module_path(resolved_object.module_id))
                        .cloned()
                        .unwrap_or_else(|| module_path.to_path_buf());
                    if resolved_object.symbol.kind != HirSymbolKind::Object {
                        diagnostics.push(Diagnostic {
                            path: module_path.to_path_buf(),
                            line: object.span.line,
                            column: object.span.column,
                            message: format!(
                                "owner object `{}` must resolve to an `obj`, found `{}`",
                                object.local_name,
                                resolved_object.symbol.kind.as_str()
                            ),
                        });
                    } else {
                        let _ = collect_object_lifecycle_surface(
                            workspace,
                            object_resolved_module,
                            &object_module_path,
                            resolved_object.symbol,
                            diagnostics,
                        );
                    }
                }
            }
            let owner_context_types = collect_owner_activation_context_types(
                workspace,
                resolved_workspace,
                resolved_module,
                objects,
            );
            if let Some(owner_context_type) = context_type {
                for actual in owner_context_types {
                    if actual.render() != owner_context_type.render() {
                        diagnostics.push(Diagnostic {
                            path: module_path.to_path_buf(),
                            line: symbol.span.line,
                            column: symbol.span.column,
                            message: format!(
                                "owner `{}` declares context `{}` but owned lifecycle hook uses `{}`",
                                symbol.name,
                                owner_context_type.render(),
                                actual.render()
                            ),
                        });
                    }
                }
            } else if !owner_context_types.is_empty() {
                diagnostics.push(Diagnostic {
                    path: module_path.to_path_buf(),
                    line: symbol.span.line,
                    column: symbol.span.column,
                    message: format!(
                        "owner `{}` must declare `context: ...` to use lifecycle context hooks",
                        symbol.name
                    ),
                });
            }
            let mut seen_exit_names = BTreeSet::new();
            for owner_exit in exits {
                if !seen_exit_names.insert(owner_exit.name.clone()) {
                    diagnostics.push(Diagnostic {
                        path: module_path.to_path_buf(),
                        line: owner_exit.span.line,
                        column: owner_exit.span.column,
                        message: format!(
                            "owner exit `{}` is declared more than once",
                            owner_exit.name
                        ),
                    });
                }
                for hold in &owner_exit.holds {
                    if !objects.iter().any(|object| object.local_name == *hold) {
                        diagnostics.push(Diagnostic {
                            path: module_path.to_path_buf(),
                            line: owner_exit.span.line,
                            column: owner_exit.span.column,
                            message: format!(
                                "owner exit `{}` holds unknown object `{hold}`",
                                owner_exit.name
                            ),
                        });
                    }
                }
            }
        }
        HirSymbolBody::Trait {
            assoc_types,
            methods,
        } => {
            let trait_scope = scope
                .with_assoc_types(assoc_types.iter().map(|assoc_type| assoc_type.name.clone()))
                .with_self();
            for assoc_type in assoc_types {
                if let Some(default_ty) = &assoc_type.default_ty {
                    validate_type_surface(
                        workspace,
                        resolved_module,
                        module_path,
                        &trait_scope,
                        default_ty,
                        assoc_type.span,
                        &format!("associated type default `{}`", assoc_type.name),
                        diagnostics,
                    );
                }
            }
            for method in methods {
                validate_symbol_surface_types(
                    workspace,
                    resolved_workspace,
                    resolved_module,
                    module_path,
                    method,
                    &trait_scope,
                    diagnostics,
                );
            }
        }
    }
}

fn validate_impl_surface_types(
    workspace: &HirWorkspaceSummary,
    resolved_workspace: &HirResolvedWorkspace,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    impl_decl: &HirImplDecl,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let scope = TypeScope::default()
        .with_params(&impl_decl.type_params)
        .with_assoc_types(
            impl_decl
                .assoc_types
                .iter()
                .map(|assoc_type| assoc_type.name.clone()),
        )
        .with_self();
    let assoc_bindings = impl_decl
        .assoc_types
        .iter()
        .filter_map(|assoc_type| {
            assoc_type
                .value_ty
                .as_ref()
                .map(|value_ty| (assoc_type.name.clone(), value_ty.clone()))
        })
        .collect::<BTreeMap<_, _>>();

    if let Some(trait_path) = &impl_decl.trait_path {
        validate_trait_surface(
            workspace,
            resolved_module,
            module_path,
            &scope,
            trait_path,
            impl_decl.span,
            "impl trait path",
            diagnostics,
        );
    }
    validate_type_surface(
        workspace,
        resolved_module,
        module_path,
        &scope,
        &impl_decl.target_type,
        impl_decl.span,
        "impl target type",
        diagnostics,
    );
    for assoc_type in &impl_decl.assoc_types {
        if let Some(value_ty) = &assoc_type.value_ty {
            validate_type_surface(
                workspace,
                resolved_module,
                module_path,
                &scope,
                value_ty,
                assoc_type.span,
                &format!("associated type binding `{}`", assoc_type.name),
                diagnostics,
            );
        }
    }
    validate_impl_trait_where_requirements_structured(
        workspace,
        resolved_workspace,
        resolved_module,
        module_path,
        impl_decl,
        &scope,
        diagnostics,
    );
    for method in &impl_decl.methods {
        validate_symbol_surface_types(
            workspace,
            resolved_workspace,
            resolved_module,
            module_path,
            method,
            &scope,
            diagnostics,
        );
        let method_scope = scope.with_params(&method.type_params);
        validate_boundary_symbol_contract(
            workspace,
            resolved_workspace,
            resolved_module,
            module_path,
            method,
            &method_scope,
            Some(impl_decl.target_type.clone()),
            &assoc_bindings,
            diagnostics,
        );
    }
}

fn render_object_declared_type(symbol: &HirSymbol) -> HirType {
    let base = arcana_hir::HirPath {
        segments: vec![symbol.name.clone()],
        span: symbol.span,
    };
    if symbol.type_params.is_empty() {
        HirType {
            kind: arcana_hir::HirTypeKind::Path(base),
            span: symbol.span,
        }
    } else {
        HirType {
            kind: arcana_hir::HirTypeKind::Apply {
                base,
                args: symbol
                    .type_params
                    .iter()
                    .map(|param| HirType {
                        kind: arcana_hir::HirTypeKind::Path(arcana_hir::HirPath {
                            segments: vec![param.clone()],
                            span: symbol.span,
                        }),
                        span: symbol.span,
                    })
                    .collect(),
            },
            span: symbol.span,
        }
    }
}

fn classify_object_lifecycle_method(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    object_symbol: &HirSymbol,
    object_scope: &TypeScope,
    method: &HirSymbol,
) -> Result<Option<(LifecycleHookSlot, Option<HirType>)>, String> {
    let slot = match method.name.as_str() {
        "init" => {
            if method.params.len() == 1 {
                LifecycleHookSlot::Init
            } else if method.params.len() == 2 {
                LifecycleHookSlot::InitWithContext
            } else {
                return Err(
                    "object lifecycle hook `init` must take `edit self` with optional `read ctx`"
                        .to_string(),
                );
            }
        }
        "resume" => {
            if method.params.len() == 1 {
                LifecycleHookSlot::Resume
            } else if method.params.len() == 2 {
                LifecycleHookSlot::ResumeWithContext
            } else {
                return Err(
                    "object lifecycle hook `resume` must take `edit self` with optional `read ctx`"
                        .to_string(),
                );
            }
        }
        _ => return Ok(None),
    };

    if method.is_async {
        return Err(format!(
            "object lifecycle hook `{}` must not be async",
            method.name
        ));
    }
    if !method.type_params.is_empty() {
        return Err(format!(
            "object lifecycle hook `{}` must not declare type parameters",
            method.name
        ));
    }
    if let Some(return_type) = &method.return_type {
        let mut semantics = SemanticArena::default();
        let unit_ty = HirType {
            kind: arcana_hir::HirTypeKind::Path(arcana_hir::HirPath {
                segments: vec!["Unit".to_string()],
                span: method.span,
            }),
            span: method.span,
        };
        if semantics.type_id_for_hir(workspace, resolved_module, object_scope, return_type)
            != semantics.type_id_for_hir(workspace, resolved_module, object_scope, &unit_ty)
        {
            return Err(format!(
                "object lifecycle hook `{}` must return Unit",
                method.name
            ));
        }
    }
    let Some(receiver) = method.params.first() else {
        return Err(format!(
            "object lifecycle hook `{}` must declare `edit self`",
            method.name
        ));
    };
    if receiver.name != "self" {
        return Err(format!(
            "object lifecycle hook `{}` must use `self` as its first parameter",
            method.name
        ));
    }
    if receiver.mode != Some(arcana_hir::HirParamMode::Edit) {
        return Err(format!(
            "object lifecycle hook `{}` must take `edit self`",
            method.name
        ));
    }
    let mut semantics = SemanticArena::default();
    let self_ty = HirType {
        kind: arcana_hir::HirTypeKind::Path(arcana_hir::HirPath {
            segments: vec!["Self".to_string()],
            span: method.span,
        }),
        span: method.span,
    };
    let expected_self = render_object_declared_type(object_symbol);
    let actual_self =
        semantics.type_id_for_hir(workspace, resolved_module, object_scope, &receiver.ty);
    let self_id = semantics.type_id_for_hir(workspace, resolved_module, object_scope, &self_ty);
    let expected_self_id =
        semantics.type_id_for_hir(workspace, resolved_module, object_scope, &expected_self);
    if actual_self != self_id && actual_self != expected_self_id {
        return Err(format!(
            "object lifecycle hook `{}` must use receiver type `Self` or `{}`",
            method.name,
            expected_self.render()
        ));
    }

    let context_type = match method.params.get(1) {
        Some(context) => {
            if context.mode != Some(arcana_hir::HirParamMode::Read) {
                return Err(format!(
                    "object lifecycle hook `{}` context parameter must be `read`",
                    method.name
                ));
            }
            Some(context.ty.clone())
        }
        None => None,
    };

    Ok(Some((slot, context_type)))
}

fn collect_object_lifecycle_surface(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    symbol: &HirSymbol,
    diagnostics: &mut Vec<Diagnostic>,
) -> ObjectLifecycleSurface {
    let object_scope = TypeScope::default()
        .with_params(&symbol.type_params)
        .with_self();
    let HirSymbolBody::Object { methods, .. } = &symbol.body else {
        return ObjectLifecycleSurface::default();
    };

    let mut surface = ObjectLifecycleSurface::default();
    let mut seen_slots = BTreeSet::new();
    for method in methods {
        match classify_object_lifecycle_method(
            workspace,
            resolved_module,
            symbol,
            &object_scope,
            method,
        ) {
            Ok(Some((slot, context_type))) => {
                if !seen_slots.insert(slot) {
                    diagnostics.push(Diagnostic {
                        path: module_path.to_path_buf(),
                        line: method.span.line,
                        column: method.span.column,
                        message: format!(
                            "object lifecycle hook `{}` is declared more than once for the same activation shape",
                            method.name
                        ),
                    });
                    continue;
                }
                match slot {
                    LifecycleHookSlot::InitWithContext => {
                        surface.init_context_type = context_type;
                    }
                    LifecycleHookSlot::ResumeWithContext => {
                        surface.resume_context_type = context_type;
                    }
                    LifecycleHookSlot::Init | LifecycleHookSlot::Resume => {}
                }
            }
            Ok(None) => {}
            Err(message) => diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: method.span.line,
                column: method.span.column,
                message,
            }),
        }
    }
    surface
}

fn collect_owner_activation_context_types(
    workspace: &HirWorkspaceSummary,
    resolved_workspace: &HirResolvedWorkspace,
    resolved_module: &HirResolvedModule,
    objects: &[arcana_hir::HirOwnerObject],
) -> Vec<HirType> {
    let mut semantics = SemanticArena::default();
    let mut context_types = BTreeMap::<TypeId, HirType>::new();
    for object in objects {
        let Some(resolved_object) =
            lookup_symbol_path(workspace, resolved_module, &object.type_path)
        else {
            continue;
        };
        let object_resolved_module = resolved_workspace
            .package(resolved_object.package_name)
            .and_then(|package| package.module(resolved_object.module_id))
            .unwrap_or(resolved_module);
        let HirSymbolBody::Object { methods, .. } = &resolved_object.symbol.body else {
            continue;
        };
        let object_scope = TypeScope::default()
            .with_params(&resolved_object.symbol.type_params)
            .with_self();
        for method in methods {
            let Ok(classified) = classify_object_lifecycle_method(
                workspace,
                object_resolved_module,
                resolved_object.symbol,
                &object_scope,
                method,
            ) else {
                continue;
            };
            if let Some((_, Some(context_type))) = classified {
                let type_id = semantics.type_id_for_hir(
                    workspace,
                    object_resolved_module,
                    &object_scope,
                    &context_type,
                );
                context_types.entry(type_id).or_insert(context_type);
            }
        }
    }
    context_types.into_values().collect()
}

fn resolve_available_owner_binding(
    workspace: &HirWorkspaceSummary,
    _resolved_workspace: &HirResolvedWorkspace,
    resolved_module: &HirResolvedModule,
    path: &[String],
) -> Option<AvailableOwnerBinding> {
    let resolved = lookup_symbol_path(workspace, resolved_module, path)?;
    let HirSymbolBody::Owner {
        objects,
        context_type,
        exits,
    } = &resolved.symbol.body
    else {
        return None;
    };
    Some(AvailableOwnerBinding {
        local_name: resolved.symbol.name.clone(),
        owner_path: canonical_symbol_path(resolved.module_id, &resolved.symbol.name),
        objects: objects
            .iter()
            .map(|object| AvailableOwnerObjectBinding {
                local_name: object.local_name.clone(),
                ty: canonical_type_from_path(
                    workspace,
                    resolved_module,
                    &object.type_path,
                    resolved.symbol.span,
                ),
            })
            .collect(),
        exit_names: exits
            .iter()
            .map(|owner_exit| owner_exit.name.clone())
            .collect(),
        activation_context_type: context_type.clone(),
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum GateShape {
    Bool,
    Option { payload: HirType },
    Result { ok: HirType, err: HirType },
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ConstructTargetShape {
    Record { fields: BTreeMap<String, HirType> },
    Variant { payload: HirType },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ConstructContributionMode {
    Direct,
    OptionPayload,
    ResultPayload,
}

fn type_bool_shape(ty: &HirType) -> bool {
    matches!(&ty.kind, HirTypeKind::Path(path) if path.segments.last().map(String::as_str) == Some("Bool"))
}

fn type_option_payload(ty: &HirType) -> Option<HirType> {
    let HirTypeKind::Apply { base, args } = &ty.kind else {
        return None;
    };
    (base.segments.last().map(String::as_str) == Some("Option") && args.len() == 1)
        .then(|| args[0].clone())
}

fn type_result_payloads(ty: &HirType) -> Option<(HirType, HirType)> {
    let HirTypeKind::Apply { base, args } = &ty.kind else {
        return None;
    };
    (base.segments.last().map(String::as_str) == Some("Result") && args.len() == 2)
        .then(|| (args[0].clone(), args[1].clone()))
}

fn infer_gate_shape(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    type_scope: &TypeScope,
    scope: &ValueScope,
    expr: &HirExpr,
) -> Option<GateShape> {
    let ty = infer_expr_value_type(workspace, resolved_module, type_scope, scope, expr)?;
    if type_bool_shape(&ty) {
        Some(GateShape::Bool)
    } else if let Some(payload) = type_option_payload(&ty) {
        Some(GateShape::Option { payload })
    } else {
        type_result_payloads(&ty).map(|(ok, err)| GateShape::Result { ok, err })
    }
}

fn infer_payload_binding(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    type_scope: &TypeScope,
    scope: &ValueScope,
    expr: &HirExpr,
) -> Option<(OwnershipClass, HirType)> {
    let payload = match infer_gate_shape(workspace, resolved_module, type_scope, scope, expr)? {
        GateShape::Option { payload } => payload,
        GateShape::Result { ok, .. } => ok,
        GateShape::Bool => return None,
    };
    Some((
        infer_type_ownership(workspace, resolved_module, type_scope, &payload),
        payload,
    ))
}

fn sync_visible_typed_local(
    scope: &mut ValueScope,
    region_scope: &mut ValueScope,
    name: &str,
    mutable: bool,
    ownership: OwnershipClass,
    ty: HirType,
) {
    scope.insert_typed(name, mutable, ownership, Some(ty.clone()));
    region_scope.insert_typed(name, mutable, ownership, Some(ty));
}

fn sync_visible_refined_local(
    scope: &mut ValueScope,
    region_scope: &mut ValueScope,
    name: &str,
    ownership: OwnershipClass,
    ty: HirType,
) {
    scope.ownership.insert(name.to_string(), ownership);
    scope.types.insert(name.to_string(), ty.clone());
    region_scope.ownership.insert(name.to_string(), ownership);
    region_scope.types.insert(name.to_string(), ty);
}

fn canonical_hir_type_key(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    ty: &HirType,
) -> String {
    canonicalize_local_hir_type(workspace, resolved_module, ty)
        .unwrap_or_else(|| ty.clone())
        .render()
}

fn canonical_hir_type_is_unit(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    ty: &HirType,
) -> bool {
    matches!(
        canonicalize_local_hir_type(workspace, resolved_module, ty)
            .unwrap_or_else(|| ty.clone())
            .kind,
        HirTypeKind::Path(HirPath { ref segments, .. })
            if segments.last().map(String::as_str) == Some("Unit")
    )
}

fn enclosing_return_type_is_unit(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    expected_return_type: Option<&HirType>,
) -> bool {
    expected_return_type
        .is_some_and(|ty| canonical_hir_type_is_unit(workspace, resolved_module, ty))
}

fn enclosing_return_type_key(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    expected_return_type: Option<&HirType>,
) -> String {
    expected_return_type
        .map(|ty| canonical_hir_type_key(workspace, resolved_module, ty))
        .unwrap_or_else(|| "Unit".to_string())
}

fn validate_returned_expr_type(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    type_scope: &TypeScope,
    scope: &ValueScope,
    expr: &HirExpr,
    expected_return_type: Option<&HirType>,
    span: Span,
    context: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(expected_ty) = expected_return_type else {
        return;
    };
    if enclosing_return_type_is_unit(workspace, resolved_module, expected_return_type) {
        push_type_contract_diagnostic(
            module_path,
            span,
            diagnostics,
            format!("{context} is not allowed because the enclosing routine returns Unit"),
        );
        return;
    }
    let Some(actual_ty) =
        infer_expr_value_type(workspace, resolved_module, type_scope, scope, expr)
    else {
        return;
    };
    let expected_key = canonical_hir_type_key(workspace, resolved_module, expected_ty);
    if canonical_hir_type_key(workspace, resolved_module, &actual_ty) != expected_key {
        push_type_contract_diagnostic(
            module_path,
            span,
            diagnostics,
            format!("{context} must have type `{expected_key}`"),
        );
    }
}

fn validate_return_statement_type(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    type_scope: &TypeScope,
    scope: &ValueScope,
    value: Option<&HirExpr>,
    expected_return_type: Option<&HirType>,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(value) = value {
        validate_returned_expr_type(
            workspace,
            resolved_module,
            module_path,
            type_scope,
            scope,
            value,
            expected_return_type,
            span,
            "return value",
            diagnostics,
        );
    } else if expected_return_type.is_some()
        && !enclosing_return_type_is_unit(workspace, resolved_module, expected_return_type)
    {
        push_type_contract_diagnostic(
            module_path,
            span,
            diagnostics,
            format!(
                "return statement requires a value of type `{}`",
                enclosing_return_type_key(workspace, resolved_module, expected_return_type)
            ),
        );
    }
}

fn validate_return_modifier_payload_type(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    type_scope: &TypeScope,
    scope: &ValueScope,
    modifier: &arcana_hir::HirHeadedModifier,
    expected_return_type: Option<&HirType>,
    context: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !matches!(
        modifier.kind,
        arcana_hir::HirHeadedModifierKind::Keyword(HeadedModifierKeyword::Return)
    ) {
        return;
    }
    let Some(payload) = &modifier.payload else {
        return;
    };
    validate_returned_expr_type(
        workspace,
        resolved_module,
        module_path,
        type_scope,
        scope,
        payload,
        expected_return_type,
        modifier.span,
        context,
        diagnostics,
    );
}

fn validate_bare_result_return_type(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    type_scope: &TypeScope,
    scope: &ValueScope,
    failure_expr: &HirExpr,
    expected_return_type: Option<&HirType>,
    span: Span,
    context: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    validate_returned_expr_type(
        workspace,
        resolved_module,
        module_path,
        type_scope,
        scope,
        failure_expr,
        expected_return_type,
        span,
        context,
        diagnostics,
    );
}

fn validate_bind_fallback_type_semantics(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    type_scope: &TypeScope,
    scope: &ValueScope,
    line_kind: &arcana_hir::HirBindLineKind,
    modifier: &arcana_hir::HirHeadedModifier,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let expected = match (&line_kind, &modifier.kind) {
        (
            arcana_hir::HirBindLineKind::Let { name, gate, .. },
            arcana_hir::HirHeadedModifierKind::Keyword(HeadedModifierKeyword::Default),
        ) => infer_payload_binding(workspace, resolved_module, type_scope, scope, gate).map(
            |(_, ty)| {
                (
                    name.as_str(),
                    canonical_hir_type_key(workspace, resolved_module, &ty),
                    "default",
                )
            },
        ),
        (
            arcana_hir::HirBindLineKind::Assign { name, .. },
            arcana_hir::HirHeadedModifierKind::Keyword(HeadedModifierKeyword::Replace),
        ) => scope.type_of(name).map(|ty| {
            (
                name.as_str(),
                canonical_hir_type_key(workspace, resolved_module, ty),
                "replace",
            )
        }),
        _ => None,
    };
    let Some((name, expected_key, modifier_name)) = expected else {
        return;
    };
    let Some(payload) = &modifier.payload else {
        return;
    };
    let Some(actual_ty) =
        infer_expr_value_type(workspace, resolved_module, type_scope, scope, payload)
    else {
        return;
    };
    let actual_key = canonical_hir_type_key(workspace, resolved_module, &actual_ty);
    if actual_key != expected_key {
        diagnostics.push(Diagnostic {
            path: module_path.to_path_buf(),
            line: modifier.span.line,
            column: modifier.span.column,
            message: format!(
                "`bind -{modifier_name}` fallback for `{name}` must have type `{expected_key}`"
            ),
        });
    }
}

fn validate_bind_refinement_stability_semantics(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    type_scope: &TypeScope,
    scope: &ValueScope,
    line_kind: &arcana_hir::HirBindLineKind,
    modifier: &arcana_hir::HirHeadedModifier,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let (name, gate, modifier_name) = match (&line_kind, &modifier.kind) {
        (
            arcana_hir::HirBindLineKind::Assign { name, gate },
            arcana_hir::HirHeadedModifierKind::Keyword(HeadedModifierKeyword::Preserve),
        ) => (name.as_str(), gate, "preserve"),
        (
            arcana_hir::HirBindLineKind::Assign { name, gate },
            arcana_hir::HirHeadedModifierKind::Keyword(HeadedModifierKeyword::Replace),
        ) => (name.as_str(), gate, "replace"),
        _ => return,
    };
    let Some(existing_ty) = scope.type_of(name) else {
        return;
    };
    let Some((_, payload_ty)) =
        infer_payload_binding(workspace, resolved_module, type_scope, scope, gate)
    else {
        return;
    };
    let expected_key = canonical_hir_type_key(workspace, resolved_module, existing_ty);
    if canonical_hir_type_key(workspace, resolved_module, &payload_ty) != expected_key {
        diagnostics.push(Diagnostic {
            path: module_path.to_path_buf(),
            line: modifier.span.line,
            column: modifier.span.column,
            message: format!(
                "`bind -{modifier_name}` payload for `{name}` must have type `{expected_key}`"
            ),
        });
    }
}

fn infer_construct_contribution_mode(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    type_scope: &TypeScope,
    scope: &ValueScope,
    expr: &HirExpr,
    target_ty: &HirType,
) -> Option<ConstructContributionMode> {
    let expected = canonicalize_local_hir_type(workspace, resolved_module, target_ty)?;
    let expected_key = expected.render();
    let actual = infer_expr_value_type(workspace, resolved_module, type_scope, scope, expr)
        .and_then(|ty| canonicalize_local_hir_type(workspace, resolved_module, &ty))?;
    if actual.render() == expected_key {
        return Some(ConstructContributionMode::Direct);
    }
    if type_option_payload(&actual).is_some_and(|payload| payload.render() == expected_key) {
        return Some(ConstructContributionMode::OptionPayload);
    }
    if type_result_payloads(&actual).is_some_and(|(ok, _)| ok.render() == expected_key) {
        return Some(ConstructContributionMode::ResultPayload);
    }
    None
}

fn module_memory_spec_binding(
    module: &HirModuleSummary,
    name: &str,
) -> Option<VisibleMemorySpecBinding> {
    module
        .memory_specs
        .iter()
        .find(|spec| spec.name == name)
        .map(|spec| VisibleMemorySpecBinding {
            family: spec.family,
            span: spec.span,
        })
}

fn lookup_memory_spec_in_package_module(
    package: &HirWorkspacePackage,
    module: &HirModuleSummary,
    path: &[String],
) -> Option<VisibleMemorySpecBinding> {
    if path.is_empty() {
        return None;
    }
    if path.len() == 1 {
        return module_memory_spec_binding(module, &path[0]);
    }
    let (spec_name, module_tail) = path.split_last()?;
    let base_relative = module
        .module_id
        .split('.')
        .skip(1)
        .map(str::to_string)
        .collect::<Vec<_>>();
    let mut target_relative = base_relative;
    target_relative.extend(module_tail.iter().cloned());
    let target_module = package.resolve_relative_module(&target_relative)?;
    module_memory_spec_binding(target_module, spec_name)
}

fn lookup_memory_spec_from_resolved_target(
    workspace: &HirWorkspaceSummary,
    target: &HirResolvedTarget,
    tail: &[String],
) -> Option<VisibleMemorySpecBinding> {
    let HirResolvedTarget::Module {
        package_id,
        module_id,
        ..
    } = target
    else {
        return None;
    };
    let package = workspace.package_by_id(package_id)?;
    let module = package.module(module_id)?;
    lookup_memory_spec_in_package_module(package, module, tail)
}

fn lookup_visible_memory_spec_binding(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    path: &[String],
) -> Option<VisibleMemorySpecBinding> {
    let current_package = current_workspace_package_for_module(workspace, resolved_module)?;
    let current_module = current_package.module(&resolved_module.module_id)?;
    if path.len() == 1 {
        return module_memory_spec_binding(current_module, &path[0]);
    }
    if let Some(binding) = resolved_module.bindings.get(&path[0])
        && let Some(spec) =
            lookup_memory_spec_from_resolved_target(workspace, &binding.target, &path[1..])
    {
        return Some(spec);
    }
    if let Some(package) = visible_package_root_for_module(workspace, resolved_module, &path[0]) {
        let root_module = package.module(&package.summary.package_name)?;
        return lookup_memory_spec_in_package_module(package, root_module, &path[1..]);
    }
    lookup_memory_spec_in_package_module(current_package, current_module, path)
}

fn resolve_construct_target_shape(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    path: &[String],
) -> Option<ConstructTargetShape> {
    if let Some(resolved) = lookup_symbol_path(workspace, resolved_module, path)
        && let HirSymbolBody::Record { fields } = &resolved.symbol.body
    {
        return Some(ConstructTargetShape::Record {
            fields: fields
                .iter()
                .map(|field| (field.name.clone(), field.ty.clone()))
                .collect(),
        });
    }
    let (variant_name, enum_path) = path.split_last()?;
    let resolved = lookup_symbol_path(workspace, resolved_module, enum_path)?;
    let HirSymbolBody::Enum { variants } = &resolved.symbol.body else {
        return None;
    };
    let variant = variants
        .iter()
        .find(|variant| variant.name == *variant_name)?;
    Some(ConstructTargetShape::Variant {
        payload: variant.payload.clone()?,
    })
}

fn resolve_record_target_fields(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    path: &[String],
) -> Option<BTreeMap<String, HirType>> {
    let resolved = lookup_symbol_path(workspace, resolved_module, path)?;
    let HirSymbolBody::Record { fields } = &resolved.symbol.body else {
        return None;
    };
    Some(
        fields
            .iter()
            .map(|field| (field.name.clone(), field.ty.clone()))
            .collect(),
    )
}

fn resolve_construct_result_type(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    path: &[String],
) -> Option<HirType> {
    resolve_construct_target_shape(workspace, resolved_module, path)?;
    let canonical_path = if lookup_symbol_path(workspace, resolved_module, path).is_some() {
        path.to_vec()
    } else {
        path[..path.len().checked_sub(1)?].to_vec()
    };
    Some(canonical_type_from_path(
        workspace,
        resolved_module,
        &canonical_path,
        Span::default(),
    ))
}

fn resolve_record_result_type(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    path: &[String],
) -> Option<HirType> {
    resolve_record_target_fields(workspace, resolved_module, path)?;
    Some(canonical_type_from_path(
        workspace,
        resolved_module,
        path,
        Span::default(),
    ))
}

fn resolve_record_fields_for_type(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    ty: &HirType,
) -> Option<BTreeMap<String, HirType>> {
    let path = match &ty.kind {
        arcana_hir::HirTypeKind::Path(path) | arcana_hir::HirTypeKind::Apply { base: path, .. } => {
            &path.segments
        }
        _ => return None,
    };
    resolve_record_target_fields(workspace, resolved_module, path)
}

fn canonicalize_local_hir_type(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    ty: &HirType,
) -> Option<HirType> {
    let package = current_workspace_package_for_module(workspace, resolved_module)?;
    let module = package.module(&resolved_module.module_id)?;
    Some(canonicalize_hir_type_in_module(
        workspace, package, module, ty,
    ))
}

fn apply_availability_attachments_to_scope(
    workspace: &HirWorkspaceSummary,
    resolved_workspace: &HirResolvedWorkspace,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    attachments: &[arcana_hir::HirAvailabilityAttachment],
    scope: &mut ValueScope,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for attachment in attachments {
        let Some(resolved) = lookup_symbol_path(workspace, resolved_module, &attachment.path)
        else {
            diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: attachment.span.line,
                column: attachment.span.column,
                message: format!(
                    "unresolved availability attachment `{}`",
                    attachment.path.join(".")
                ),
            });
            continue;
        };
        match resolved.symbol.kind {
            HirSymbolKind::Owner => {
                if let Some(owner) = resolve_available_owner_binding(
                    workspace,
                    resolved_workspace,
                    resolved_module,
                    &attachment.path,
                ) {
                    scope.attach_owner(owner);
                }
            }
            HirSymbolKind::Object => {
                scope.attach_object_name(resolved.symbol.name.clone());
            }
            other => diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: attachment.span.line,
                column: attachment.span.column,
                message: format!(
                    "availability attachment `{}` must resolve to an owner or object, found `{}`",
                    attachment.path.join("."),
                    other.as_str()
                ),
            }),
        }
    }
}

fn validate_symbol_value_semantics(
    workspace: &HirWorkspaceSummary,
    resolved_workspace: &HirResolvedWorkspace,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    symbol: &HirSymbol,
    inherited_type_scope: &TypeScope,
    inherited_scope: &ValueScope,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let type_scope = inherited_type_scope.with_params(&symbol.type_params);
    let mut scope = inherited_scope.with_symbol_params(&symbol.params);
    scope.enclosing_return_type = symbol.return_type.clone();
    apply_availability_attachments_to_scope(
        workspace,
        resolved_workspace,
        resolved_module,
        module_path,
        &symbol.availability,
        &mut scope,
        diagnostics,
    );
    for param in &symbol.params {
        let ownership = infer_type_ownership(workspace, resolved_module, &type_scope, &param.ty);
        scope.ownership.insert(param.name.clone(), ownership);
        scope.types.insert(param.name.clone(), param.ty.clone());
    }
    let mut symbol_cleanup_candidates = symbol
        .params
        .iter()
        .filter_map(|param| {
            scope
                .binding_id_of(&param.name)
                .map(|binding_id| CleanupFooterCandidate {
                    name: param.name.clone(),
                    binding_id,
                    ownership: scope.ownership_of(&param.name),
                    ty: scope.type_of(&param.name).cloned(),
                })
        })
        .collect::<Vec<_>>();
    symbol_cleanup_candidates.extend(collect_cleanup_footer_candidates_recursive(
        workspace,
        resolved_workspace,
        resolved_module,
        module_path,
        &type_scope,
        &scope,
        &symbol.statements,
        true,
    ));
    let symbol_cleanup_policy = validate_cleanup_footer_targets(
        workspace,
        resolved_module,
        module_path,
        &symbol.cleanup_footers,
        &symbol_cleanup_candidates,
        diagnostics,
    );
    let mut borrow_state = BorrowFlowState::default();
    for param in &symbol.params {
        if should_activate_cleanup_binding(
            workspace,
            resolved_module,
            &scope,
            &symbol_cleanup_policy,
            &param.name,
        ) && let Some(binding_id) = scope.binding_id_of(&param.name)
        {
            borrow_state.activate_cleanup_binding(binding_id);
        }
    }
    validate_rollup_handlers(
        workspace,
        resolved_module,
        module_path,
        &symbol.cleanup_footers,
        diagnostics,
    );
    validate_statement_block_semantics(
        workspace,
        resolved_workspace,
        resolved_module,
        module_path,
        &symbol.statements,
        &type_scope,
        &mut scope,
        &mut borrow_state,
        &symbol_cleanup_policy,
        symbol.return_type.as_ref(),
        diagnostics,
    );

    if let HirSymbolBody::Owner {
        objects,
        context_type,
        exits,
    } = &symbol.body
    {
        let owner_path = canonical_symbol_path(&resolved_module.module_id, &symbol.name);
        let mut owner_scope = scope.clone();
        owner_scope.insert_typed(&symbol.name, false, OwnershipClass::Copy, None);
        let available_owner = AvailableOwnerBinding {
            local_name: symbol.name.clone(),
            owner_path,
            objects: objects
                .iter()
                .map(|object| AvailableOwnerObjectBinding {
                    local_name: object.local_name.clone(),
                    ty: canonical_type_from_path(
                        workspace,
                        resolved_module,
                        &object.type_path,
                        symbol.span,
                    ),
                })
                .collect(),
            exit_names: exits
                .iter()
                .map(|owner_exit| owner_exit.name.clone())
                .collect(),
            activation_context_type: context_type.clone(),
        };
        let _ = owner_scope.activate_owner(&available_owner, None, false);
        for owner_exit in exits {
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                &type_scope,
                &owner_scope,
                &owner_exit.condition,
                owner_exit.span,
                diagnostics,
            );
            validate_expected_expr_type(
                module_path,
                &owner_exit.condition,
                owner_exit.span,
                diagnostics,
                ExprTypeClass::Bool,
                "owner exit condition",
            );
        }
    }

    if let HirSymbolBody::Trait {
        assoc_types,
        methods,
    } = &symbol.body
    {
        let trait_scope = type_scope
            .with_assoc_types(assoc_types.iter().map(|assoc_type| assoc_type.name.clone()))
            .with_self();
        for method in methods {
            validate_symbol_value_semantics(
                workspace,
                resolved_workspace,
                resolved_module,
                module_path,
                method,
                &trait_scope,
                &ValueScope::default(),
                diagnostics,
            );
        }
    }
    if let HirSymbolBody::Object { methods, .. } = &symbol.body {
        let object_scope = type_scope.with_self();
        for method in methods {
            validate_symbol_value_semantics(
                workspace,
                resolved_workspace,
                resolved_module,
                module_path,
                method,
                &object_scope,
                &ValueScope::default(),
                diagnostics,
            );
        }
    }
}

fn validate_impl_value_semantics(
    workspace: &HirWorkspaceSummary,
    resolved_workspace: &HirResolvedWorkspace,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    impl_decl: &HirImplDecl,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let scope = TypeScope::default()
        .with_params(&impl_decl.type_params)
        .with_assoc_types(
            impl_decl
                .assoc_types
                .iter()
                .map(|assoc_type| assoc_type.name.clone()),
        )
        .with_self();
    for method in &impl_decl.methods {
        validate_symbol_value_semantics(
            workspace,
            resolved_workspace,
            resolved_module,
            module_path,
            method,
            &scope,
            &ValueScope::default(),
            diagnostics,
        );
    }
}

fn validate_rollup_handlers(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    cleanup_footers: &[arcana_hir::HirCleanupFooter],
    diagnostics: &mut Vec<Diagnostic>,
) {
    if cleanup_footers
        .iter()
        .any(|rollup| rollup.handler_path.is_empty() || has_bare_cleanup_rollup(cleanup_footers))
        && let Err(message) = resolve_cleanup_contract_trait_path(workspace, resolved_module)
    {
        diagnostics.push(Diagnostic {
            path: module_path.to_path_buf(),
            line: cleanup_footers[0].span.line,
            column: cleanup_footers[0].span.column,
            message,
        });
    }
    for rollup in cleanup_footers {
        if rollup.handler_path.is_empty() {
            continue;
        }
        let Some(symbol_ref) = lookup_symbol_path(workspace, resolved_module, &rollup.handler_path)
        else {
            diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: rollup.span.line,
                column: rollup.span.column,
                message: format!(
                    "unresolved cleanup footer handler `{}`",
                    rollup.handler_path.join(".")
                ),
            });
            continue;
        };
        if !matches!(
            symbol_ref.symbol.kind,
            arcana_hir::HirSymbolKind::Fn
                | arcana_hir::HirSymbolKind::System
                | arcana_hir::HirSymbolKind::Behavior
                | arcana_hir::HirSymbolKind::Const
        ) {
            diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: rollup.span.line,
                column: rollup.span.column,
                message: format!(
                    "cleanup footer handler `{}` must resolve to a callable symbol",
                    rollup.handler_path.join(".")
                ),
            });
            continue;
        }
        if symbol_ref.symbol.is_async {
            diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: rollup.span.line,
                column: rollup.span.column,
                message: format!(
                    "cleanup footer handler `{}` cannot be async in v1",
                    rollup.handler_path.join(".")
                ),
            });
            continue;
        }
        if symbol_ref.symbol.params.len() != 1 {
            diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: rollup.span.line,
                column: rollup.span.column,
                message: format!(
                    "cleanup footer handler `{}` must accept exactly one parameter in v1",
                    rollup.handler_path.join(".")
                ),
            });
            continue;
        }
        if symbol_ref.symbol.params[0].mode != Some(arcana_hir::HirParamMode::Take) {
            diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: rollup.span.line,
                column: rollup.span.column,
                message: format!(
                    "cleanup footer handler `{}` must take its target parameter in v1",
                    rollup.handler_path.join(".")
                ),
            });
        }
        if !symbol_return_type_is_cleanup_result(symbol_ref.symbol.return_type.as_ref()) {
            diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: rollup.span.line,
                column: rollup.span.column,
                message: format!(
                    "cleanup footer handler `{}` must return `Result[Unit, Str]` in v1",
                    rollup.handler_path.join(".")
                ),
            });
        }
    }
}

#[derive(Clone, Debug, Default)]
struct CleanupFooterPolicy {
    cover_all_cleanup_capable: bool,
    explicit_target_names: BTreeSet<String>,
}

#[derive(Clone, Debug)]
struct CleanupFooterCandidate {
    name: String,
    binding_id: u64,
    ownership: OwnershipClass,
    ty: Option<HirType>,
}

fn has_bare_cleanup_rollup(cleanup_footers: &[arcana_hir::HirCleanupFooter]) -> bool {
    cleanup_footers
        .iter()
        .any(|rollup| rollup.subject.is_empty())
}

fn push_cleanup_footer_candidate(
    candidates: &mut Vec<CleanupFooterCandidate>,
    scope: &ValueScope,
    name: &str,
) {
    let Some(binding_id) = scope.binding_id_of(name) else {
        return;
    };
    candidates.push(CleanupFooterCandidate {
        name: name.to_string(),
        binding_id,
        ownership: scope.ownership_of(name),
        ty: scope.type_of(name).cloned(),
    });
}

fn infer_iterable_binding_type(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    type_scope: &TypeScope,
    scope: &ValueScope,
    iterable: &HirExpr,
) -> Option<HirType> {
    let iterable_ty =
        infer_expr_value_type(workspace, resolved_module, type_scope, scope, iterable)?;
    let path_segments_match = |segments: &[String], expected: &[&str]| {
        segments.len() == expected.len()
            && segments
                .iter()
                .map(String::as_str)
                .zip(expected.iter().copied())
                .all(|(actual, expected)| actual == expected)
    };
    match &iterable_ty.kind {
        HirTypeKind::Path(path) if path.segments.len() == 1 && path.segments[0] == "RangeInt" => {
            Some(HirType {
                kind: HirTypeKind::Path(arcana_hir::HirPath {
                    segments: vec!["Int".to_string()],
                    span: Span::default(),
                }),
                span: Span::default(),
            })
        }
        HirTypeKind::Apply { base, args }
            if matches!(&base.segments[..], [name] if name == "List" || name == "Array")
                || path_segments_match(&base.segments, &["std", "collections", "list", "List"])
                || path_segments_match(
                    &base.segments,
                    &["std", "collections", "array", "Array"],
                ) =>
        {
            args.first().cloned()
        }
        HirTypeKind::Apply { base, args }
            if matches!(&base.segments[..], [name] if name == "Map")
                || path_segments_match(&base.segments, &["std", "collections", "map", "Map"]) =>
        {
            match (args.first(), args.get(1)) {
                (Some(key), Some(value)) => Some(HirType {
                    kind: HirTypeKind::Tuple(vec![key.clone(), value.clone()]),
                    span: Span::default(),
                }),
                _ => None,
            }
        }
        _ => None,
    }
}

fn collect_cleanup_footer_candidates_recursive(
    workspace: &HirWorkspaceSummary,
    resolved_workspace: &HirResolvedWorkspace,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    type_scope: &TypeScope,
    scope: &ValueScope,
    statements: &[HirStatement],
    collect_bindings: bool,
) -> Vec<CleanupFooterCandidate> {
    let mut candidates = Vec::new();
    let mut body_scope = scope.clone();
    for statement in statements {
        match &statement.kind {
            HirStatementKind::Let {
                mutable,
                name,
                value,
            } => {
                if let Some(owner_activation) = resolve_owner_activation_expr(
                    workspace,
                    resolved_workspace,
                    resolved_module,
                    value,
                ) {
                    let mut inserted_names =
                        body_scope.activate_owner(&owner_activation.owner, Some(name), *mutable);
                    for object in &owner_activation.owner.objects {
                        if !body_scope
                            .attached_object_names
                            .contains(&object.local_name)
                        {
                            continue;
                        }
                        if body_scope.binding_id_of(&object.local_name).is_none() {
                            body_scope.insert_typed(
                                &object.local_name,
                                true,
                                OwnershipClass::Move,
                                Some(object.ty.clone()),
                            );
                        }
                        if !inserted_names
                            .iter()
                            .any(|inserted_name| inserted_name == &object.local_name)
                        {
                            inserted_names.push(object.local_name.clone());
                        }
                    }
                    for inserted_name in inserted_names {
                        if collect_bindings {
                            push_cleanup_footer_candidate(
                                &mut candidates,
                                &body_scope,
                                &inserted_name,
                            );
                        }
                    }
                    continue;
                }
                let ownership = infer_expr_ownership(
                    workspace,
                    resolved_module,
                    type_scope,
                    &body_scope,
                    value,
                );
                let ty = infer_expr_value_type(
                    workspace,
                    resolved_module,
                    type_scope,
                    &body_scope,
                    value,
                );
                let inserted = bind_pattern_into_scope(
                    workspace,
                    resolved_module,
                    type_scope,
                    &mut body_scope,
                    name,
                    *mutable,
                    ty,
                )
                .unwrap_or_else(|_| {
                    let mut names = Vec::new();
                    if let Some(pattern) = parse_binding_pattern(name) {
                        collect_binding_pattern_names(&pattern, &mut names);
                    }
                    for binding_name in &names {
                        body_scope.insert_typed(
                            binding_name,
                            *mutable,
                            ownership,
                            None,
                        );
                    }
                    names
                });
                if collect_bindings {
                    for inserted_name in inserted {
                        push_cleanup_footer_candidate(
                            &mut candidates,
                            &body_scope,
                            &inserted_name,
                        );
                    }
                }
            }
            HirStatementKind::Expr { expr } => {
                if let Some(owner_activation) = resolve_owner_activation_expr(
                    workspace,
                    resolved_workspace,
                    resolved_module,
                    expr,
                ) {
                    let mut inserted_names =
                        body_scope.activate_owner(&owner_activation.owner, None, false);
                    for object in &owner_activation.owner.objects {
                        if !body_scope
                            .attached_object_names
                            .contains(&object.local_name)
                        {
                            continue;
                        }
                        if body_scope.binding_id_of(&object.local_name).is_none() {
                            body_scope.insert_typed(
                                &object.local_name,
                                true,
                                OwnershipClass::Move,
                                Some(object.ty.clone()),
                            );
                        }
                        if !inserted_names
                            .iter()
                            .any(|inserted_name| inserted_name == &object.local_name)
                        {
                            inserted_names.push(object.local_name.clone());
                        }
                    }
                    for inserted_name in inserted_names {
                        if collect_bindings {
                            push_cleanup_footer_candidate(
                                &mut candidates,
                                &body_scope,
                                &inserted_name,
                            );
                        }
                    }
                }
            }
            HirStatementKind::If {
                then_branch,
                else_branch,
                ..
            } => {
                let nested_collect = collect_bindings && statement.cleanup_footers.is_empty();
                let mut then_scope = body_scope.clone();
                let mut ignored = Vec::new();
                apply_availability_attachments_to_scope(
                    workspace,
                    resolved_workspace,
                    resolved_module,
                    module_path,
                    &statement.availability,
                    &mut then_scope,
                    &mut ignored,
                );
                candidates.extend(collect_cleanup_footer_candidates_recursive(
                    workspace,
                    resolved_workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    &then_scope,
                    then_branch,
                    nested_collect,
                ));
                if let Some(else_branch) = else_branch {
                    let mut else_scope = body_scope.clone();
                    let mut ignored = Vec::new();
                    apply_availability_attachments_to_scope(
                        workspace,
                        resolved_workspace,
                        resolved_module,
                        module_path,
                        &statement.availability,
                        &mut else_scope,
                        &mut ignored,
                    );
                    candidates.extend(collect_cleanup_footer_candidates_recursive(
                        workspace,
                        resolved_workspace,
                        resolved_module,
                        module_path,
                        type_scope,
                        &else_scope,
                        else_branch,
                        nested_collect,
                    ));
                }
            }
            HirStatementKind::While { body, .. } => {
                let nested_collect = collect_bindings && statement.cleanup_footers.is_empty();
                let mut nested_scope = body_scope.clone();
                let mut ignored = Vec::new();
                apply_availability_attachments_to_scope(
                    workspace,
                    resolved_workspace,
                    resolved_module,
                    module_path,
                    &statement.availability,
                    &mut nested_scope,
                    &mut ignored,
                );
                candidates.extend(collect_cleanup_footer_candidates_recursive(
                    workspace,
                    resolved_workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    &nested_scope,
                    body,
                    nested_collect,
                ));
            }
            HirStatementKind::For {
                binding,
                iterable,
                body,
            } => {
                let nested_collect = collect_bindings && statement.cleanup_footers.is_empty();
                let mut nested_scope = body_scope.clone();
                let iterable_binding_ty = infer_iterable_binding_type(
                    workspace,
                    resolved_module,
                    type_scope,
                    &body_scope,
                    iterable,
                );
                let iterable_binding_ownership = iterable_binding_ty
                    .as_ref()
                    .map(|ty| infer_type_ownership(workspace, resolved_module, type_scope, ty))
                    .unwrap_or_default();
                let inserted = bind_pattern_into_scope(
                    workspace,
                    resolved_module,
                    type_scope,
                    &mut nested_scope,
                    binding,
                    false,
                    iterable_binding_ty,
                )
                .unwrap_or_else(|_| {
                    let mut names = Vec::new();
                    if let Some(pattern) = parse_binding_pattern(binding) {
                        collect_binding_pattern_names(&pattern, &mut names);
                    }
                    for binding_name in &names {
                        nested_scope.insert_typed(
                            binding_name,
                            false,
                            iterable_binding_ownership,
                            None,
                        );
                    }
                    names
                });
                if nested_collect {
                    for binding_name in inserted {
                        push_cleanup_footer_candidate(&mut candidates, &nested_scope, &binding_name);
                    }
                }
                let mut ignored = Vec::new();
                apply_availability_attachments_to_scope(
                    workspace,
                    resolved_workspace,
                    resolved_module,
                    module_path,
                    &statement.availability,
                    &mut nested_scope,
                    &mut ignored,
                );
                candidates.extend(collect_cleanup_footer_candidates_recursive(
                    workspace,
                    resolved_workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    &nested_scope,
                    body,
                    nested_collect,
                ));
            }
            _ => {}
        }
    }
    candidates
}

fn cleanup_target_supports_default_cleanup_contract(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    ownership: OwnershipClass,
    ty: Option<&HirType>,
) -> bool {
    if !ownership.is_move_only() {
        return false;
    }
    let Some(ty) = ty else {
        return false;
    };
    let Ok(contract_path) = resolve_cleanup_contract_trait_path(workspace, resolved_module) else {
        return false;
    };
    let candidates =
        lookup_method_candidates_for_hir_type(workspace, resolved_module, ty, "cleanup")
            .into_iter()
            .filter(|candidate| candidate.trait_path.as_ref() == Some(&contract_path))
            .collect::<Vec<_>>();
    matches!(candidates.as_slice(), [_])
}

fn validate_cleanup_footer_targets(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    cleanup_footers: &[arcana_hir::HirCleanupFooter],
    candidates: &[CleanupFooterCandidate],
    diagnostics: &mut Vec<Diagnostic>,
) -> CleanupFooterPolicy {
    let mut policy = CleanupFooterPolicy {
        cover_all_cleanup_capable: has_bare_cleanup_rollup(cleanup_footers),
        explicit_target_names: BTreeSet::new(),
    };
    let mut candidates_by_name = BTreeMap::<&str, Vec<&CleanupFooterCandidate>>::new();
    for candidate in candidates {
        candidates_by_name
            .entry(candidate.name.as_str())
            .or_default()
            .push(candidate);
    }
    for rollup in cleanup_footers {
        if rollup.subject.is_empty() {
            continue;
        }
        let Some(matches) = candidates_by_name.get(rollup.subject.as_str()) else {
            diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: rollup.span.line,
                column: rollup.span.column,
                message: format!(
                    "cleanup footer target `{}` is not available in the owning header scope",
                    rollup.subject
                ),
            });
            continue;
        };
        let mut distinct_binding_ids = matches
            .iter()
            .map(|candidate| candidate.binding_id)
            .collect::<BTreeSet<_>>();
        if distinct_binding_ids.len() > 1 {
            diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: rollup.span.line,
                column: rollup.span.column,
                message: format!(
                    "cleanup footer target `{}` is ambiguous in the owning header scope",
                    rollup.subject
                ),
            });
            continue;
        }
        let candidate = matches[0];
        if !cleanup_target_supports_default_cleanup_contract(
            workspace,
            resolved_module,
            candidate.ownership,
            candidate.ty.as_ref(),
        ) {
            diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: rollup.span.line,
                column: rollup.span.column,
                message: format!(
                    "cleanup footer target `{}` is not cleanup-capable in the owning header scope",
                    rollup.subject
                ),
            });
            continue;
        }
        distinct_binding_ids.clear();
        policy.explicit_target_names.insert(candidate.name.clone());
    }
    policy
}

fn symbol_return_type_is_cleanup_result(return_type: Option<&HirType>) -> bool {
    let Some(return_type) = return_type else {
        return false;
    };
    let HirTypeKind::Apply { base, args } = &return_type.kind else {
        return false;
    };
    if args.len() != 2 {
        return false;
    }
    let result_root = base.segments.last().map(String::as_str);
    let ok_root = match &args[0].kind {
        HirTypeKind::Path(path) => path.segments.last().map(String::as_str),
        _ => None,
    };
    let err_root = match &args[1].kind {
        HirTypeKind::Path(path) => path.segments.last().map(String::as_str),
        _ => None,
    };
    result_root == Some("Result") && ok_root == Some("Unit") && err_root == Some("Str")
}

fn resolve_cleanup_contract_trait_path(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
) -> Result<Vec<String>, String> {
    let mut found = BTreeSet::<Vec<String>>::new();
    for package in workspace.packages.values() {
        for module in &package.summary.modules {
            for lang_item in &module.lang_items {
                if lang_item.name != "cleanup_contract" {
                    continue;
                }
                let Some(symbol_ref) =
                    lookup_symbol_path(workspace, resolved_module, &lang_item.target)
                else {
                    continue;
                };
                if symbol_ref.symbol.kind != HirSymbolKind::Trait {
                    continue;
                }
                let mut path = symbol_ref
                    .module_id
                    .split('.')
                    .map(ToString::to_string)
                    .collect::<Vec<_>>();
                path.push(symbol_ref.symbol.name.clone());
                found.insert(path);
            }
        }
    }
    match found.into_iter().collect::<Vec<_>>().as_slice() {
        [] => Err("no `cleanup_contract` lang item is available for cleanup footers".to_string()),
        [path] => Ok(path.clone()),
        paths => Err(format!(
            "`cleanup_contract` is ambiguous; candidates: {}",
            paths
                .iter()
                .map(|path| path.join("."))
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

fn binding_supports_default_cleanup_contract(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    scope: &ValueScope,
    name: &str,
) -> bool {
    cleanup_target_supports_default_cleanup_contract(
        workspace,
        resolved_module,
        scope.ownership_of(name),
        scope.type_of(name),
    )
}

fn should_activate_cleanup_binding(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    scope: &ValueScope,
    policy: &CleanupFooterPolicy,
    name: &str,
) -> bool {
    policy.explicit_target_names.contains(name)
        || (policy.cover_all_cleanup_capable
            && binding_supports_default_cleanup_contract(workspace, resolved_module, scope, name))
}

fn activate_current_cleanup_binding(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    borrow_state: &mut BorrowFlowState,
    scope: &ValueScope,
    current_block_cleanup_policy: &CleanupFooterPolicy,
    name: &str,
) {
    if !should_activate_cleanup_binding(
        workspace,
        resolved_module,
        scope,
        current_block_cleanup_policy,
        name,
    ) {
        return;
    }
    if let Some(binding_id) = scope.binding_id_of(name) {
        borrow_state.activate_cleanup_binding(binding_id);
    }
}

fn validate_statement_block_semantics(
    workspace: &HirWorkspaceSummary,
    resolved_workspace: &HirResolvedWorkspace,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    statements: &[HirStatement],
    type_scope: &TypeScope,
    scope: &mut ValueScope,
    borrow_state: &mut BorrowFlowState,
    current_block_cleanup_policy: &CleanupFooterPolicy,
    expected_return_type: Option<&HirType>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for statement in statements {
        validate_rollup_handlers(
            workspace,
            resolved_module,
            module_path,
            &statement.cleanup_footers,
            diagnostics,
        );
        match &statement.kind {
            HirStatementKind::Let {
                mutable,
                name,
                value,
            } => {
                let destructuring_binding = binding_pattern_is_destructuring(name);
                if let Some(owner_activation) = resolve_owner_activation_expr(
                    workspace,
                    resolved_workspace,
                    resolved_module,
                    value,
                ) {
                    if destructuring_binding {
                        diagnostics.push(Diagnostic {
                            path: module_path.to_path_buf(),
                            line: statement.span.line,
                            column: statement.span.column,
                            message: "owner activation bindings must use a simple name".to_string(),
                        });
                        continue;
                    }
                    if let Some(ref message) = owner_activation.invalid {
                        diagnostics.push(Diagnostic {
                            path: module_path.to_path_buf(),
                            line: statement.span.line,
                            column: statement.span.column,
                            message: message.clone(),
                        });
                    }
                    if let Some(context) = owner_activation.context {
                        validate_expr_semantics(
                            workspace,
                            resolved_module,
                            module_path,
                            type_scope,
                            scope,
                            context,
                            statement.span,
                            diagnostics,
                        );
                        validate_expr_borrow_flow(
                            workspace,
                            resolved_module,
                            type_scope,
                            module_path,
                            scope,
                            context,
                            statement.span,
                            borrow_state,
                            diagnostics,
                        );
                        note_expr_moves(
                            workspace,
                            resolved_module,
                            type_scope,
                            scope,
                            context,
                            borrow_state,
                        );
                        note_escaping_expr_borrows(borrow_state, context, scope);
                    }
                    validate_owner_activation_context(
                        workspace,
                        resolved_workspace,
                        resolved_module,
                        module_path,
                        type_scope,
                        scope,
                        &owner_activation,
                        statement.span,
                        diagnostics,
                    );
                    let inserted =
                        scope.activate_owner(&owner_activation.owner, Some(name), *mutable);
                    for inserted_name in inserted {
                        activate_current_cleanup_binding(
                            workspace,
                            resolved_module,
                            borrow_state,
                            scope,
                            current_block_cleanup_policy,
                            &inserted_name,
                        );
                    }
                    continue;
                }
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    value,
                    statement.span,
                    diagnostics,
                );
                validate_expr_borrow_flow(
                    workspace,
                    resolved_module,
                    type_scope,
                    module_path,
                    scope,
                    value,
                    statement.span,
                    borrow_state,
                    diagnostics,
                );
                note_expr_moves(
                    workspace,
                    resolved_module,
                    type_scope,
                    scope,
                    value,
                    borrow_state,
                );
                note_escaping_expr_borrows(borrow_state, value, scope);
                let ty =
                    infer_expr_value_type(workspace, resolved_module, type_scope, scope, value);
                match bind_pattern_into_scope(
                    workspace,
                    resolved_module,
                    type_scope,
                    scope,
                    name,
                    *mutable,
                    ty,
                ) {
                    Ok(inserted) => {
                        for inserted_name in inserted {
                            borrow_state.clear_local(&inserted_name);
                            activate_current_cleanup_binding(
                                workspace,
                                resolved_module,
                                borrow_state,
                                scope,
                                current_block_cleanup_policy,
                                &inserted_name,
                            );
                        }
                    }
                    Err(message) => diagnostics.push(Diagnostic {
                        path: module_path.to_path_buf(),
                        line: statement.span.line,
                        column: statement.span.column,
                        message,
                    }),
                }
            }
            HirStatementKind::Return { value } => {
                if let Some(value) = value {
                    validate_expr_semantics(
                        workspace,
                        resolved_module,
                        module_path,
                        type_scope,
                        scope,
                        value,
                        statement.span,
                        diagnostics,
                    );
                    validate_expr_borrow_flow(
                        workspace,
                        resolved_module,
                        type_scope,
                        module_path,
                        scope,
                        value,
                        statement.span,
                        borrow_state,
                        diagnostics,
                    );
                    validate_return_borrow_ties(
                        module_path,
                        scope,
                        value,
                        statement.span,
                        diagnostics,
                    );
                }
                validate_return_statement_type(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    value.as_ref(),
                    expected_return_type,
                    statement.span,
                    diagnostics,
                );
            }
            HirStatementKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    condition,
                    statement.span,
                    diagnostics,
                );
                validate_expr_borrow_flow(
                    workspace,
                    resolved_module,
                    type_scope,
                    module_path,
                    scope,
                    condition,
                    statement.span,
                    borrow_state,
                    diagnostics,
                );
                note_expr_moves(
                    workspace,
                    resolved_module,
                    type_scope,
                    scope,
                    condition,
                    borrow_state,
                );
                validate_expected_expr_type(
                    module_path,
                    condition,
                    statement.span,
                    diagnostics,
                    ExprTypeClass::Bool,
                    "if condition",
                );
                let mut then_scope = scope.clone();
                apply_availability_attachments_to_scope(
                    workspace,
                    resolved_workspace,
                    resolved_module,
                    module_path,
                    &statement.availability,
                    &mut then_scope,
                    diagnostics,
                );
                let statement_has_own_cleanup = !statement.cleanup_footers.is_empty();
                let mut nested_cleanup_candidates = if statement_has_own_cleanup {
                    collect_cleanup_footer_candidates_recursive(
                        workspace,
                        resolved_workspace,
                        resolved_module,
                        module_path,
                        type_scope,
                        &then_scope,
                        then_branch,
                        true,
                    )
                } else {
                    Vec::new()
                };
                let nested_cleanup_policy = if statement_has_own_cleanup {
                    validate_cleanup_footer_targets(
                        workspace,
                        resolved_module,
                        module_path,
                        &statement.cleanup_footers,
                        &nested_cleanup_candidates,
                        diagnostics,
                    )
                } else {
                    current_block_cleanup_policy.clone()
                };
                let mut then_borrows = borrow_state.clone();
                validate_statement_block_semantics(
                    workspace,
                    resolved_workspace,
                    resolved_module,
                    module_path,
                    then_branch,
                    type_scope,
                    &mut then_scope,
                    &mut then_borrows,
                    &nested_cleanup_policy,
                    expected_return_type,
                    diagnostics,
                );
                borrow_state.merge_moves_from(&then_borrows);
                if let Some(else_branch) = else_branch {
                    let mut else_scope = scope.clone();
                    apply_availability_attachments_to_scope(
                        workspace,
                        resolved_workspace,
                        resolved_module,
                        module_path,
                        &statement.availability,
                        &mut else_scope,
                        diagnostics,
                    );
                    if statement_has_own_cleanup {
                        nested_cleanup_candidates.extend(
                            collect_cleanup_footer_candidates_recursive(
                                workspace,
                                resolved_workspace,
                                resolved_module,
                                module_path,
                                type_scope,
                                &else_scope,
                                else_branch,
                                true,
                            ),
                        );
                    }
                    let mut else_borrows = borrow_state.clone();
                    validate_statement_block_semantics(
                        workspace,
                        resolved_workspace,
                        resolved_module,
                        module_path,
                        else_branch,
                        type_scope,
                        &mut else_scope,
                        &mut else_borrows,
                        &nested_cleanup_policy,
                        expected_return_type,
                        diagnostics,
                    );
                    borrow_state.merge_moves_from(&else_borrows);
                }
            }
            HirStatementKind::While { condition, body } => {
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    condition,
                    statement.span,
                    diagnostics,
                );
                validate_expr_borrow_flow(
                    workspace,
                    resolved_module,
                    type_scope,
                    module_path,
                    scope,
                    condition,
                    statement.span,
                    borrow_state,
                    diagnostics,
                );
                note_expr_moves(
                    workspace,
                    resolved_module,
                    type_scope,
                    scope,
                    condition,
                    borrow_state,
                );
                validate_expected_expr_type(
                    module_path,
                    condition,
                    statement.span,
                    diagnostics,
                    ExprTypeClass::Bool,
                    "while condition",
                );
                let mut body_scope = scope.clone();
                body_scope.loop_depth += 1;
                apply_availability_attachments_to_scope(
                    workspace,
                    resolved_workspace,
                    resolved_module,
                    module_path,
                    &statement.availability,
                    &mut body_scope,
                    diagnostics,
                );
                let statement_has_own_cleanup = !statement.cleanup_footers.is_empty();
                let nested_cleanup_candidates = if statement_has_own_cleanup {
                    collect_cleanup_footer_candidates_recursive(
                        workspace,
                        resolved_workspace,
                        resolved_module,
                        module_path,
                        type_scope,
                        &body_scope,
                        body,
                        true,
                    )
                } else {
                    Vec::new()
                };
                let nested_cleanup_policy = if statement_has_own_cleanup {
                    validate_cleanup_footer_targets(
                        workspace,
                        resolved_module,
                        module_path,
                        &statement.cleanup_footers,
                        &nested_cleanup_candidates,
                        diagnostics,
                    )
                } else {
                    current_block_cleanup_policy.clone()
                };
                let mut body_borrows = borrow_state.clone();
                validate_statement_block_semantics(
                    workspace,
                    resolved_workspace,
                    resolved_module,
                    module_path,
                    body,
                    type_scope,
                    &mut body_scope,
                    &mut body_borrows,
                    &nested_cleanup_policy,
                    expected_return_type,
                    diagnostics,
                );
                borrow_state.merge_moves_from(&body_borrows);
            }
            HirStatementKind::For {
                binding,
                iterable,
                body,
            } => {
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    iterable,
                    statement.span,
                    diagnostics,
                );
                validate_expr_borrow_flow(
                    workspace,
                    resolved_module,
                    type_scope,
                    module_path,
                    scope,
                    iterable,
                    statement.span,
                    borrow_state,
                    diagnostics,
                );
                note_expr_moves(
                    workspace,
                    resolved_module,
                    type_scope,
                    scope,
                    iterable,
                    borrow_state,
                );
                let mut body_scope = scope.clone();
                body_scope.loop_depth += 1;
                let iterable_binding_ty = infer_iterable_binding_type(
                    workspace,
                    resolved_module,
                    type_scope,
                    scope,
                    iterable,
                );
                let inserted_bindings = match bind_pattern_into_scope(
                    workspace,
                    resolved_module,
                    type_scope,
                    &mut body_scope,
                    binding,
                    false,
                    iterable_binding_ty,
                ) {
                    Ok(inserted) => inserted,
                    Err(message) => {
                        diagnostics.push(Diagnostic {
                            path: module_path.to_path_buf(),
                            line: statement.span.line,
                            column: statement.span.column,
                            message,
                        });
                        Vec::new()
                    }
                };
                apply_availability_attachments_to_scope(
                    workspace,
                    resolved_workspace,
                    resolved_module,
                    module_path,
                    &statement.availability,
                    &mut body_scope,
                    diagnostics,
                );
                let statement_has_own_cleanup = !statement.cleanup_footers.is_empty();
                let mut nested_cleanup_candidates = if statement_has_own_cleanup {
                    inserted_bindings
                        .iter()
                        .filter_map(|binding_name| {
                            body_scope.binding_id_of(binding_name).map(|binding_id| {
                                CleanupFooterCandidate {
                                    name: binding_name.clone(),
                                    binding_id,
                                    ownership: body_scope.ownership_of(binding_name),
                                    ty: body_scope.type_of(binding_name).cloned(),
                                }
                            })
                        })
                        .collect::<Vec<_>>()
                } else {
                    Vec::new()
                };
                if statement_has_own_cleanup {
                    nested_cleanup_candidates.extend(collect_cleanup_footer_candidates_recursive(
                        workspace,
                        resolved_workspace,
                        resolved_module,
                        module_path,
                        type_scope,
                        &body_scope,
                        body,
                        true,
                    ));
                }
                let nested_cleanup_policy = if statement_has_own_cleanup {
                    validate_cleanup_footer_targets(
                        workspace,
                        resolved_module,
                        module_path,
                        &statement.cleanup_footers,
                        &nested_cleanup_candidates,
                        diagnostics,
                    )
                } else {
                    current_block_cleanup_policy.clone()
                };
                let mut body_borrows = borrow_state.clone();
                for binding_name in &inserted_bindings {
                    activate_current_cleanup_binding(
                        workspace,
                        resolved_module,
                        &mut body_borrows,
                        &body_scope,
                        &nested_cleanup_policy,
                        binding_name,
                    );
                }
                validate_statement_block_semantics(
                    workspace,
                    resolved_workspace,
                    resolved_module,
                    module_path,
                    body,
                    type_scope,
                    &mut body_scope,
                    &mut body_borrows,
                    &nested_cleanup_policy,
                    expected_return_type,
                    diagnostics,
                );
                for binding_name in inserted_bindings {
                    body_borrows.clear_local(&binding_name);
                }
                borrow_state.merge_moves_from(&body_borrows);
            }
            HirStatementKind::Defer { expr } | HirStatementKind::Expr { expr } => {
                if let HirStatementKind::Expr { .. } = &statement.kind
                    && let Some(owner_activation) = resolve_owner_activation_expr(
                        workspace,
                        resolved_workspace,
                        resolved_module,
                        expr,
                    )
                {
                    if let Some(ref message) = owner_activation.invalid {
                        diagnostics.push(Diagnostic {
                            path: module_path.to_path_buf(),
                            line: statement.span.line,
                            column: statement.span.column,
                            message: message.clone(),
                        });
                    }
                    if let Some(context) = owner_activation.context {
                        validate_expr_semantics(
                            workspace,
                            resolved_module,
                            module_path,
                            type_scope,
                            scope,
                            context,
                            statement.span,
                            diagnostics,
                        );
                        validate_expr_borrow_flow(
                            workspace,
                            resolved_module,
                            type_scope,
                            module_path,
                            scope,
                            context,
                            statement.span,
                            borrow_state,
                            diagnostics,
                        );
                        note_expr_moves(
                            workspace,
                            resolved_module,
                            type_scope,
                            scope,
                            context,
                            borrow_state,
                        );
                        note_escaping_expr_borrows(borrow_state, context, scope);
                    }
                    validate_owner_activation_context(
                        workspace,
                        resolved_workspace,
                        resolved_module,
                        module_path,
                        type_scope,
                        scope,
                        &owner_activation,
                        statement.span,
                        diagnostics,
                    );
                    let inserted = scope.activate_owner(&owner_activation.owner, None, false);
                    for inserted_name in inserted {
                        activate_current_cleanup_binding(
                            workspace,
                            resolved_module,
                            borrow_state,
                            scope,
                            current_block_cleanup_policy,
                            &inserted_name,
                        );
                    }
                    continue;
                }
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    expr,
                    statement.span,
                    diagnostics,
                );
                validate_expr_borrow_flow(
                    workspace,
                    resolved_module,
                    type_scope,
                    module_path,
                    scope,
                    expr,
                    statement.span,
                    borrow_state,
                    diagnostics,
                );
                note_expr_moves(
                    workspace,
                    resolved_module,
                    type_scope,
                    scope,
                    expr,
                    borrow_state,
                );
            }
            HirStatementKind::Assign { target, value, .. } => {
                validate_assign_target_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    target,
                    statement.span,
                    diagnostics,
                );
                validate_assign_target_borrow_flow(
                    workspace,
                    resolved_module,
                    type_scope,
                    module_path,
                    scope,
                    target,
                    statement.span,
                    borrow_state,
                    diagnostics,
                );
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    value,
                    statement.span,
                    diagnostics,
                );
                validate_expr_borrow_flow(
                    workspace,
                    resolved_module,
                    type_scope,
                    module_path,
                    scope,
                    value,
                    statement.span,
                    borrow_state,
                    diagnostics,
                );
                note_expr_moves(
                    workspace,
                    resolved_module,
                    type_scope,
                    scope,
                    value,
                    borrow_state,
                );
                if let HirAssignTarget::Name { text } = target
                    && scope.contains(text)
                {
                    borrow_state.clear_local(text);
                    let ownership =
                        infer_expr_ownership(workspace, resolved_module, type_scope, scope, value);
                    let ty =
                        infer_expr_value_type(workspace, resolved_module, type_scope, scope, value);
                    scope.ownership.insert(text.clone(), ownership);
                    if let Some(ty) = ty {
                        scope.types.insert(text.clone(), ty);
                    } else {
                        scope.types.remove(text);
                    }
                }
                if matches!(target, HirAssignTarget::Name { text } if scope.contains(text)) {
                    note_escaping_expr_borrows(borrow_state, value, scope);
                }
            }
            HirStatementKind::Recycle {
                default_modifier,
                lines,
            } => {
                if default_modifier.is_none() {
                    diagnostics.push(Diagnostic {
                        path: module_path.to_path_buf(),
                        line: statement.span.line,
                        column: statement.span.column,
                        message: "recycle requires a default modifier in v1".to_string(),
                    });
                }
                let mut region_scope = scope.clone();
                region_scope.headed_region_depth += 1;
                if let Some(modifier) = default_modifier
                    && let Some(payload) = &modifier.payload
                {
                    validate_expr_semantics(
                        workspace,
                        resolved_module,
                        module_path,
                        type_scope,
                        &region_scope,
                        payload,
                        statement.span,
                        diagnostics,
                    );
                    validate_return_modifier_payload_type(
                        workspace,
                        resolved_module,
                        module_path,
                        type_scope,
                        &region_scope,
                        modifier,
                        expected_return_type,
                        "`recycle -return` payload",
                        diagnostics,
                    );
                }
                for line in lines {
                    let gate_shape = match &line.kind {
                        arcana_hir::HirRecycleLineKind::Expr { gate }
                        | arcana_hir::HirRecycleLineKind::Let { gate, .. }
                        | arcana_hir::HirRecycleLineKind::Assign { gate, .. } => {
                            validate_expr_semantics(
                                workspace,
                                resolved_module,
                                module_path,
                                type_scope,
                                &region_scope,
                                gate,
                                line.span,
                                diagnostics,
                            );
                            validate_expr_borrow_flow(
                                workspace,
                                resolved_module,
                                type_scope,
                                module_path,
                                &region_scope,
                                gate,
                                line.span,
                                borrow_state,
                                diagnostics,
                            );
                            note_expr_moves(
                                workspace,
                                resolved_module,
                                type_scope,
                                &region_scope,
                                gate,
                                borrow_state,
                            );
                            infer_gate_shape(
                                workspace,
                                resolved_module,
                                type_scope,
                                &region_scope,
                                gate,
                            )
                        }
                    };
                    if let Some(modifier) = &line.modifier
                        && let Some(payload) = &modifier.payload
                    {
                        validate_expr_semantics(
                            workspace,
                            resolved_module,
                            module_path,
                            type_scope,
                            &region_scope,
                            payload,
                            line.span,
                            diagnostics,
                        );
                        validate_return_modifier_payload_type(
                            workspace,
                            resolved_module,
                            module_path,
                            type_scope,
                            &region_scope,
                            modifier,
                            expected_return_type,
                            "`recycle -return` payload",
                            diagnostics,
                        );
                    }
                    if let Some(modifier) = line.modifier.as_ref().or(default_modifier.as_ref()) {
                        validate_recycle_modifier_semantics(
                            module_path,
                            &region_scope,
                            modifier,
                            gate_shape.as_ref(),
                            diagnostics,
                        );
                        if modifier.payload.is_none()
                            && matches!(gate_shape, Some(GateShape::Result { .. }))
                        {
                            let failure_expr = match &line.kind {
                                arcana_hir::HirRecycleLineKind::Expr { gate }
                                | arcana_hir::HirRecycleLineKind::Let { gate, .. }
                                | arcana_hir::HirRecycleLineKind::Assign { gate, .. } => gate,
                            };
                            validate_bare_result_return_type(
                                workspace,
                                resolved_module,
                                module_path,
                                type_scope,
                                &region_scope,
                                failure_expr,
                                expected_return_type,
                                modifier.span,
                                "bare `recycle -return` failure",
                                diagnostics,
                            );
                        }
                    }
                    match &line.kind {
                        arcana_hir::HirRecycleLineKind::Let {
                            mutable,
                            name,
                            gate,
                        } => {
                            if let Some((ownership, ty)) = infer_payload_binding(
                                workspace,
                                resolved_module,
                                type_scope,
                                &region_scope,
                                gate,
                            ) {
                                note_escaping_expr_borrows(borrow_state, gate, &region_scope);
                                borrow_state.clear_local(name);
                                sync_visible_typed_local(
                                    scope,
                                    &mut region_scope,
                                    name,
                                    *mutable,
                                    ownership,
                                    ty,
                                );
                                activate_current_cleanup_binding(
                                    workspace,
                                    resolved_module,
                                    borrow_state,
                                    scope,
                                    current_block_cleanup_policy,
                                    name,
                                );
                            } else {
                                diagnostics.push(Diagnostic {
                                    path: module_path.to_path_buf(),
                                    line: line.span.line,
                                    column: line.span.column,
                                    message: "payload-bearing recycle lines require Option or Result gates".to_string(),
                                });
                            }
                        }
                        arcana_hir::HirRecycleLineKind::Assign { name, gate } => {
                            let target = HirAssignTarget::Name { text: name.clone() };
                            validate_assign_target_semantics(
                                workspace,
                                resolved_module,
                                module_path,
                                type_scope,
                                &region_scope,
                                &target,
                                line.span,
                                diagnostics,
                            );
                            validate_assign_target_borrow_flow(
                                workspace,
                                resolved_module,
                                type_scope,
                                module_path,
                                &region_scope,
                                &target,
                                line.span,
                                borrow_state,
                                diagnostics,
                            );
                            if region_scope.contains(name) {
                                if let Some((ownership, ty)) = infer_payload_binding(
                                    workspace,
                                    resolved_module,
                                    type_scope,
                                    &region_scope,
                                    gate,
                                ) {
                                    note_escaping_expr_borrows(borrow_state, gate, &region_scope);
                                    borrow_state.clear_local(name);
                                    sync_visible_refined_local(
                                        scope,
                                        &mut region_scope,
                                        name,
                                        ownership,
                                        ty,
                                    );
                                } else {
                                    diagnostics.push(Diagnostic {
                                        path: module_path.to_path_buf(),
                                        line: line.span.line,
                                        column: line.span.column,
                                        message: "payload-bearing recycle lines require Option or Result gates".to_string(),
                                    });
                                }
                            }
                        }
                        arcana_hir::HirRecycleLineKind::Expr { .. } => {
                            if gate_shape.is_none() {
                                diagnostics.push(Diagnostic {
                                    path: module_path.to_path_buf(),
                                    line: line.span.line,
                                    column: line.span.column,
                                    message:
                                        "recycle gates must evaluate to Bool, Option, or Result"
                                            .to_string(),
                                });
                            }
                        }
                    }
                }
            }
            HirStatementKind::Bind {
                default_modifier,
                lines,
            } => {
                if default_modifier.is_none() {
                    diagnostics.push(Diagnostic {
                        path: module_path.to_path_buf(),
                        line: statement.span.line,
                        column: statement.span.column,
                        message: "bind requires a default modifier in v1".to_string(),
                    });
                }
                let mut region_scope = scope.clone();
                region_scope.headed_region_depth += 1;
                if let Some(modifier) = default_modifier
                    && let Some(payload) = &modifier.payload
                {
                    validate_expr_semantics(
                        workspace,
                        resolved_module,
                        module_path,
                        type_scope,
                        &region_scope,
                        payload,
                        statement.span,
                        diagnostics,
                    );
                    validate_return_modifier_payload_type(
                        workspace,
                        resolved_module,
                        module_path,
                        type_scope,
                        &region_scope,
                        modifier,
                        expected_return_type,
                        "`bind -return` payload",
                        diagnostics,
                    );
                }
                for line in lines {
                    let gate_shape = match &line.kind {
                        arcana_hir::HirBindLineKind::Let { gate, .. }
                        | arcana_hir::HirBindLineKind::Assign { gate, .. } => {
                            validate_expr_semantics(
                                workspace,
                                resolved_module,
                                module_path,
                                type_scope,
                                &region_scope,
                                gate,
                                line.span,
                                diagnostics,
                            );
                            validate_expr_borrow_flow(
                                workspace,
                                resolved_module,
                                type_scope,
                                module_path,
                                &region_scope,
                                gate,
                                line.span,
                                borrow_state,
                                diagnostics,
                            );
                            note_expr_moves(
                                workspace,
                                resolved_module,
                                type_scope,
                                &region_scope,
                                gate,
                                borrow_state,
                            );
                            infer_gate_shape(
                                workspace,
                                resolved_module,
                                type_scope,
                                &region_scope,
                                gate,
                            )
                        }
                        arcana_hir::HirBindLineKind::Require { expr } => {
                            validate_expr_semantics(
                                workspace,
                                resolved_module,
                                module_path,
                                type_scope,
                                &region_scope,
                                expr,
                                line.span,
                                diagnostics,
                            );
                            validate_expected_expr_type(
                                module_path,
                                expr,
                                line.span,
                                diagnostics,
                                ExprTypeClass::Bool,
                                "bind require condition",
                            );
                            None
                        }
                    };
                    if let Some(modifier) = &line.modifier
                        && let Some(payload) = &modifier.payload
                    {
                        validate_expr_semantics(
                            workspace,
                            resolved_module,
                            module_path,
                            type_scope,
                            &region_scope,
                            payload,
                            line.span,
                            diagnostics,
                        );
                        validate_return_modifier_payload_type(
                            workspace,
                            resolved_module,
                            module_path,
                            type_scope,
                            &region_scope,
                            modifier,
                            expected_return_type,
                            "`bind -return` payload",
                            diagnostics,
                        );
                    }
                    if let Some(modifier) = line.modifier.as_ref().or(default_modifier.as_ref()) {
                        validate_bind_modifier_semantics(
                            module_path,
                            &region_scope,
                            modifier,
                            gate_shape.as_ref(),
                            &line.kind,
                            diagnostics,
                        );
                        validate_bind_fallback_type_semantics(
                            workspace,
                            resolved_module,
                            module_path,
                            type_scope,
                            &region_scope,
                            &line.kind,
                            modifier,
                            diagnostics,
                        );
                        validate_bind_refinement_stability_semantics(
                            workspace,
                            resolved_module,
                            module_path,
                            type_scope,
                            &region_scope,
                            &line.kind,
                            modifier,
                            diagnostics,
                        );
                        if modifier.payload.is_none()
                            && matches!(gate_shape, Some(GateShape::Result { .. }))
                            && !matches!(line.kind, arcana_hir::HirBindLineKind::Require { .. })
                        {
                            let failure_expr = match &line.kind {
                                arcana_hir::HirBindLineKind::Let { gate, .. }
                                | arcana_hir::HirBindLineKind::Assign { gate, .. } => gate,
                                arcana_hir::HirBindLineKind::Require { .. } => unreachable!(),
                            };
                            validate_bare_result_return_type(
                                workspace,
                                resolved_module,
                                module_path,
                                type_scope,
                                &region_scope,
                                failure_expr,
                                expected_return_type,
                                modifier.span,
                                "bare `bind -return` failure",
                                diagnostics,
                            );
                        }
                    }
                    match &line.kind {
                        arcana_hir::HirBindLineKind::Let {
                            mutable,
                            name,
                            gate,
                        } => {
                            if let Some((ownership, ty)) = infer_payload_binding(
                                workspace,
                                resolved_module,
                                type_scope,
                                &region_scope,
                                gate,
                            ) {
                                note_escaping_expr_borrows(borrow_state, gate, &region_scope);
                                borrow_state.clear_local(name);
                                sync_visible_typed_local(
                                    scope,
                                    &mut region_scope,
                                    name,
                                    *mutable,
                                    ownership,
                                    ty,
                                );
                                activate_current_cleanup_binding(
                                    workspace,
                                    resolved_module,
                                    borrow_state,
                                    scope,
                                    current_block_cleanup_policy,
                                    name,
                                );
                            } else {
                                diagnostics.push(Diagnostic {
                                    path: module_path.to_path_buf(),
                                    line: line.span.line,
                                    column: line.span.column,
                                    message: "bind payload lines require Option or Result gates"
                                        .to_string(),
                                });
                            }
                        }
                        arcana_hir::HirBindLineKind::Assign { name, gate } => {
                            let target = HirAssignTarget::Name { text: name.clone() };
                            validate_assign_target_semantics(
                                workspace,
                                resolved_module,
                                module_path,
                                type_scope,
                                &region_scope,
                                &target,
                                line.span,
                                diagnostics,
                            );
                            validate_assign_target_borrow_flow(
                                workspace,
                                resolved_module,
                                type_scope,
                                module_path,
                                &region_scope,
                                &target,
                                line.span,
                                borrow_state,
                                diagnostics,
                            );
                            if region_scope.contains(name) {
                                if let Some((ownership, ty)) = infer_payload_binding(
                                    workspace,
                                    resolved_module,
                                    type_scope,
                                    &region_scope,
                                    gate,
                                ) {
                                    note_escaping_expr_borrows(borrow_state, gate, &region_scope);
                                    borrow_state.clear_local(name);
                                    sync_visible_refined_local(
                                        scope,
                                        &mut region_scope,
                                        name,
                                        ownership,
                                        ty,
                                    );
                                } else {
                                    diagnostics.push(Diagnostic {
                                        path: module_path.to_path_buf(),
                                        line: line.span.line,
                                        column: line.span.column,
                                        message:
                                            "bind payload lines require Option or Result gates"
                                                .to_string(),
                                    });
                                }
                            }
                        }
                        arcana_hir::HirBindLineKind::Require { .. } => {}
                    }
                }
            }
            HirStatementKind::Construct(region) => {
                let mut region_scope = scope.clone();
                region_scope.headed_region_depth += 1;
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    &region_scope,
                    &region.target,
                    region.span,
                    diagnostics,
                );
                if let Some(arcana_hir::HirConstructDestination::Place { target }) =
                    &region.destination
                {
                    validate_assign_target_semantics(
                        workspace,
                        resolved_module,
                        module_path,
                        type_scope,
                        scope,
                        target,
                        region.span,
                        diagnostics,
                    );
                    validate_assign_target_borrow_flow(
                        workspace,
                        resolved_module,
                        type_scope,
                        module_path,
                        scope,
                        target,
                        region.span,
                        borrow_state,
                        diagnostics,
                    );
                }
                if let Some(modifier) = &region.default_modifier
                    && let Some(payload) = &modifier.payload
                {
                    validate_expr_semantics(
                        workspace,
                        resolved_module,
                        module_path,
                        type_scope,
                        &region_scope,
                        payload,
                        region.span,
                        diagnostics,
                    );
                    validate_return_modifier_payload_type(
                        workspace,
                        resolved_module,
                        module_path,
                        type_scope,
                        &region_scope,
                        modifier,
                        expected_return_type,
                        "`construct -return` payload",
                        diagnostics,
                    );
                }
                for line in &region.lines {
                    validate_expr_semantics(
                        workspace,
                        resolved_module,
                        module_path,
                        type_scope,
                        &region_scope,
                        &line.value,
                        line.span,
                        diagnostics,
                    );
                    if let Some(modifier) = &line.modifier
                        && let Some(payload) = &modifier.payload
                    {
                        validate_expr_semantics(
                            workspace,
                            resolved_module,
                            module_path,
                            type_scope,
                            &region_scope,
                            payload,
                            line.span,
                            diagnostics,
                        );
                        validate_return_modifier_payload_type(
                            workspace,
                            resolved_module,
                            module_path,
                            type_scope,
                            &region_scope,
                            modifier,
                            expected_return_type,
                            "`construct -return` payload",
                            diagnostics,
                        );
                    }
                }
                validate_construct_region_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    &region_scope,
                    expected_return_type,
                    region,
                    diagnostics,
                );
                if let Some(arcana_hir::HirConstructDestination::Deliver { name }) =
                    &region.destination
                {
                    let delivered_ty =
                        flatten_callable_expr_path(&region.target).and_then(|path| {
                            resolve_construct_result_type(workspace, resolved_module, &path)
                        });
                    let ownership = delivered_ty
                        .as_ref()
                        .map(|ty| infer_type_ownership(workspace, resolved_module, type_scope, ty))
                        .unwrap_or_default();
                    borrow_state.clear_local(name);
                    scope.insert_typed(name, false, ownership, delivered_ty);
                    activate_current_cleanup_binding(
                        workspace,
                        resolved_module,
                        borrow_state,
                        scope,
                        current_block_cleanup_policy,
                        name,
                    );
                }
                if let Some(arcana_hir::HirConstructDestination::Place { target }) =
                    &region.destination
                    && let Some(name) = assign_target_root_local(target, scope)
                {
                    borrow_state.clear_local(name);
                }
            }
            HirStatementKind::Record(region) => {
                let mut region_scope = scope.clone();
                region_scope.headed_region_depth += 1;
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    &region_scope,
                    &region.target,
                    region.span,
                    diagnostics,
                );
                if let Some(base) = &region.base {
                    validate_expr_semantics(
                        workspace,
                        resolved_module,
                        module_path,
                        type_scope,
                        &region_scope,
                        base,
                        region.span,
                        diagnostics,
                    );
                }
                if let Some(arcana_hir::HirConstructDestination::Place { target }) =
                    &region.destination
                {
                    validate_assign_target_semantics(
                        workspace,
                        resolved_module,
                        module_path,
                        type_scope,
                        scope,
                        target,
                        region.span,
                        diagnostics,
                    );
                    validate_assign_target_borrow_flow(
                        workspace,
                        resolved_module,
                        type_scope,
                        module_path,
                        scope,
                        target,
                        region.span,
                        borrow_state,
                        diagnostics,
                    );
                }
                if let Some(modifier) = &region.default_modifier
                    && let Some(payload) = &modifier.payload
                {
                    validate_expr_semantics(
                        workspace,
                        resolved_module,
                        module_path,
                        type_scope,
                        &region_scope,
                        payload,
                        region.span,
                        diagnostics,
                    );
                    validate_return_modifier_payload_type(
                        workspace,
                        resolved_module,
                        module_path,
                        type_scope,
                        &region_scope,
                        modifier,
                        expected_return_type,
                        "`record -return` payload",
                        diagnostics,
                    );
                }
                for line in &region.lines {
                    validate_expr_semantics(
                        workspace,
                        resolved_module,
                        module_path,
                        type_scope,
                        &region_scope,
                        &line.value,
                        line.span,
                        diagnostics,
                    );
                    if let Some(modifier) = &line.modifier
                        && let Some(payload) = &modifier.payload
                    {
                        validate_expr_semantics(
                            workspace,
                            resolved_module,
                            module_path,
                            type_scope,
                            &region_scope,
                            payload,
                            line.span,
                            diagnostics,
                        );
                        validate_return_modifier_payload_type(
                            workspace,
                            resolved_module,
                            module_path,
                            type_scope,
                            &region_scope,
                            modifier,
                            expected_return_type,
                            "`record -return` payload",
                            diagnostics,
                        );
                    }
                }
                validate_record_region_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    &region_scope,
                    expected_return_type,
                    region,
                    diagnostics,
                );
                if let Some(arcana_hir::HirConstructDestination::Deliver { name }) =
                    &region.destination
                {
                    let delivered_ty =
                        flatten_callable_expr_path(&region.target).and_then(|path| {
                            resolve_record_result_type(workspace, resolved_module, &path)
                        });
                    let ownership = delivered_ty
                        .as_ref()
                        .map(|ty| infer_type_ownership(workspace, resolved_module, type_scope, ty))
                        .unwrap_or_default();
                    borrow_state.clear_local(name);
                    scope.insert_typed(name, false, ownership, delivered_ty);
                    activate_current_cleanup_binding(
                        workspace,
                        resolved_module,
                        borrow_state,
                        scope,
                        current_block_cleanup_policy,
                        name,
                    );
                }
                if let Some(arcana_hir::HirConstructDestination::Place { target }) =
                    &region.destination
                    && let Some(name) = assign_target_root_local(target, scope)
                {
                    borrow_state.clear_local(name);
                }
            }
            HirStatementKind::MemorySpec(spec) => {
                let mut region_scope = scope.clone();
                region_scope.headed_region_depth += 1;
                if let Some(modifier) = &spec.default_modifier
                    && let Some(payload) = &modifier.payload
                {
                    validate_expr_semantics(
                        workspace,
                        resolved_module,
                        module_path,
                        type_scope,
                        &region_scope,
                        payload,
                        spec.span,
                        diagnostics,
                    );
                }
                for detail in &spec.details {
                    if memory_detail_descriptor(spec.family, detail.key)
                        .map(|descriptor| descriptor.value_kind == MemoryDetailValueKind::IntExpr)
                        .unwrap_or(true)
                    {
                        validate_expr_semantics(
                            workspace,
                            resolved_module,
                            module_path,
                            type_scope,
                            &region_scope,
                            &detail.value,
                            detail.span,
                            diagnostics,
                        );
                    }
                    if let Some(modifier) = &detail.modifier
                        && let Some(payload) = &modifier.payload
                    {
                        validate_expr_semantics(
                            workspace,
                            resolved_module,
                            module_path,
                            type_scope,
                            &region_scope,
                            payload,
                            detail.span,
                            diagnostics,
                        );
                    }
                }
                validate_memory_spec_decl_semantics(module_path, spec, false, diagnostics);
                scope.insert_memory_spec(&spec.name, spec.family, spec.span);
            }
            HirStatementKind::Break | HirStatementKind::Continue => {}
        }
    }
}

fn validate_assign_target_semantics(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    type_scope: &TypeScope,
    scope: &ValueScope,
    target: &HirAssignTarget,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match target {
        HirAssignTarget::Name { text } => validate_value_path_segments(
            workspace,
            resolved_module,
            module_path,
            scope,
            std::slice::from_ref(text),
            span,
            "assignment target",
            diagnostics,
        ),
        target @ HirAssignTarget::MemberAccess {
            target: inner_target,
            ..
        } => {
            if let Some(path) = flatten_assign_target_path(target)
                && should_resolve_member_path_as_namespace(workspace, resolved_module, scope, &path)
            {
                validate_value_path_segments(
                    workspace,
                    resolved_module,
                    module_path,
                    scope,
                    &path,
                    span,
                    "assignment target",
                    diagnostics,
                );
                return;
            }
            validate_assign_target_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                inner_target,
                span,
                diagnostics,
            );
        }
        HirAssignTarget::Index { target, index } => {
            validate_assign_target_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                target,
                span,
                diagnostics,
            );
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                index,
                span,
                diagnostics,
            );
        }
    }
}

fn validate_assign_target_borrow_flow(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    type_scope: &TypeScope,
    module_path: &Path,
    scope: &ValueScope,
    target: &HirAssignTarget,
    span: Span,
    state: &BorrowFlowState,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(name) = assign_target_root_local(target, scope) {
        if matches!(target, HirAssignTarget::Name { .. })
            && scope
                .binding_id_of(name)
                .is_some_and(|binding_id| state.has_active_cleanup_binding(binding_id))
        {
            push_type_contract_diagnostic(
                module_path,
                span,
                diagnostics,
                format!("cleanup footer target `{name}` cannot be reassigned after activation"),
            );
        } else if !matches!(target, HirAssignTarget::Name { .. }) && state.has_moved(name) {
            push_type_contract_diagnostic(
                module_path,
                span,
                diagnostics,
                format!("use of moved local `{name}`"),
            );
        } else if state.has_any_borrow(name) {
            push_type_contract_diagnostic(
                module_path,
                span,
                diagnostics,
                format!("cannot assign to local `{name}` while it is borrowed"),
            );
        }
    }

    match target {
        HirAssignTarget::Name { .. } => {}
        HirAssignTarget::MemberAccess { target, .. } => {
            validate_assign_target_borrow_flow(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                target,
                span,
                state,
                diagnostics,
            );
        }
        HirAssignTarget::Index { target, index } => {
            validate_assign_target_borrow_flow(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                target,
                span,
                state,
                diagnostics,
            );
            validate_expr_borrow_flow(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                index,
                span,
                state,
                diagnostics,
            );
        }
    }
}

fn validate_expr_semantics(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    type_scope: &TypeScope,
    scope: &ValueScope,
    expr: &HirExpr,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match expr {
        HirExpr::Path { segments } => validate_value_path_segments(
            workspace,
            resolved_module,
            module_path,
            scope,
            segments,
            span,
            "value expression",
            diagnostics,
        ),
        HirExpr::BoolLiteral { .. } | HirExpr::IntLiteral { .. } | HirExpr::StrLiteral { .. } => {}
        HirExpr::Pair { left, right } => {
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                left,
                span,
                diagnostics,
            );
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                right,
                span,
                diagnostics,
            );
        }
        HirExpr::CollectionLiteral { items } => {
            for item in items {
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    item,
                    span,
                    diagnostics,
                );
            }
        }
        HirExpr::Match { subject, arms } => {
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                subject,
                span,
                diagnostics,
            );
            for arm in arms {
                let mut arm_scope = scope.clone();
                for pattern in &arm.patterns {
                    validate_match_pattern_semantics(
                        workspace,
                        resolved_module,
                        Some(subject),
                        pattern,
                        &mut arm_scope,
                    );
                }
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    &arm_scope,
                    &arm.value,
                    arm.span,
                    diagnostics,
                );
            }
        }
        HirExpr::ConstructRegion(region) => {
            if scope.headed_region_depth > 0 {
                diagnostics.push(Diagnostic {
                    path: module_path.to_path_buf(),
                    line: span.line,
                    column: span.column,
                    message: "headed regions cannot nest inside another headed region in v1"
                        .to_string(),
                });
            }
            let mut region_scope = scope.clone();
            region_scope.headed_region_depth += 1;
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                &region_scope,
                &region.target,
                span,
                diagnostics,
            );
            if let Some(arcana_hir::HirConstructDestination::Place { target }) = &region.destination
            {
                validate_assign_target_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    target,
                    span,
                    diagnostics,
                );
            }
            if let Some(modifier) = &region.default_modifier
                && let Some(payload) = &modifier.payload
            {
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    &region_scope,
                    payload,
                    span,
                    diagnostics,
                );
                validate_return_modifier_payload_type(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    &region_scope,
                    modifier,
                    scope.enclosing_return_type.as_ref(),
                    "`construct -return` payload",
                    diagnostics,
                );
            }
            for line in &region.lines {
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    &region_scope,
                    &line.value,
                    line.span,
                    diagnostics,
                );
                if let Some(modifier) = &line.modifier
                    && let Some(payload) = &modifier.payload
                {
                    validate_expr_semantics(
                        workspace,
                        resolved_module,
                        module_path,
                        type_scope,
                        &region_scope,
                        payload,
                        line.span,
                        diagnostics,
                    );
                    validate_return_modifier_payload_type(
                        workspace,
                        resolved_module,
                        module_path,
                        type_scope,
                        &region_scope,
                        modifier,
                        scope.enclosing_return_type.as_ref(),
                        "`construct -return` payload",
                        diagnostics,
                    );
                }
            }
            validate_construct_region_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                &region_scope,
                scope.enclosing_return_type.as_ref(),
                region,
                diagnostics,
            );
        }
        HirExpr::RecordRegion(region) => {
            if scope.headed_region_depth > 0 {
                diagnostics.push(Diagnostic {
                    path: module_path.to_path_buf(),
                    line: span.line,
                    column: span.column,
                    message: "headed regions cannot nest inside another headed region in v1"
                        .to_string(),
                });
            }
            let mut region_scope = scope.clone();
            region_scope.headed_region_depth += 1;
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                &region_scope,
                &region.target,
                span,
                diagnostics,
            );
            if let Some(base) = &region.base {
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    &region_scope,
                    base,
                    span,
                    diagnostics,
                );
            }
            if let Some(arcana_hir::HirConstructDestination::Place { target }) = &region.destination
            {
                validate_assign_target_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    target,
                    span,
                    diagnostics,
                );
            }
            if let Some(modifier) = &region.default_modifier
                && let Some(payload) = &modifier.payload
            {
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    &region_scope,
                    payload,
                    span,
                    diagnostics,
                );
                validate_return_modifier_payload_type(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    &region_scope,
                    modifier,
                    scope.enclosing_return_type.as_ref(),
                    "`record -return` payload",
                    diagnostics,
                );
            }
            for line in &region.lines {
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    &region_scope,
                    &line.value,
                    line.span,
                    diagnostics,
                );
                if let Some(modifier) = &line.modifier
                    && let Some(payload) = &modifier.payload
                {
                    validate_expr_semantics(
                        workspace,
                        resolved_module,
                        module_path,
                        type_scope,
                        &region_scope,
                        payload,
                        line.span,
                        diagnostics,
                    );
                    validate_return_modifier_payload_type(
                        workspace,
                        resolved_module,
                        module_path,
                        type_scope,
                        &region_scope,
                        modifier,
                        scope.enclosing_return_type.as_ref(),
                        "`record -return` payload",
                        diagnostics,
                    );
                }
            }
            validate_record_region_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                &region_scope,
                scope.enclosing_return_type.as_ref(),
                region,
                diagnostics,
            );
        }
        HirExpr::Chain { steps, .. } => {
            for step in steps {
                validate_chain_step_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    step,
                    span,
                    diagnostics,
                );
            }
        }
        HirExpr::MemoryPhrase {
            family,
            arena,
            init_args,
            constructor,
            attached,
        } => {
            if let Some(path) = flatten_member_expr_path(arena) {
                let resolved_as_value = (path.len() == 1 && scope.contains(&path[0]))
                    || value_path_exists(workspace, resolved_module, &path);
                if !resolved_as_value {
                    let memory_spec = if path.len() == 1 {
                        scope.memory_spec(&path[0]).cloned()
                    } else {
                        lookup_visible_memory_spec_binding(workspace, resolved_module, &path)
                    };
                    if let Some(memory_spec) = memory_spec {
                        if memory_spec.family.as_str() != family {
                            diagnostics.push(Diagnostic {
                                path: module_path.to_path_buf(),
                                line: span.line,
                                column: span.column,
                                message: format!(
                                    "memory phrase requires `{family}` but spec `{}` is `{}`",
                                    path.join("."),
                                    memory_spec.family.as_str()
                                ),
                            });
                        } else if !memory_family_descriptor(memory_spec.family)
                            .phrase_consumers
                            .contains(&"memory_phrase")
                        {
                            diagnostics.push(Diagnostic {
                                path: module_path.to_path_buf(),
                                line: span.line,
                                column: span.column,
                                message: format!(
                                    "memory family `{}` is not consumable from memory phrases in v1",
                                    memory_spec.family.as_str()
                                ),
                            });
                        }
                    } else {
                        validate_expr_semantics(
                            workspace,
                            resolved_module,
                            module_path,
                            type_scope,
                            scope,
                            arena,
                            span,
                            diagnostics,
                        );
                    }
                } else {
                    validate_expr_semantics(
                        workspace,
                        resolved_module,
                        module_path,
                        type_scope,
                        scope,
                        arena,
                        span,
                        diagnostics,
                    );
                }
            } else {
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    arena,
                    span,
                    diagnostics,
                );
            }
            for arg in init_args {
                validate_phrase_arg_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    arg,
                    span,
                    diagnostics,
                );
            }
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                constructor,
                span,
                diagnostics,
            );
            if let Some(path) = flatten_callable_expr_path(constructor) {
                validate_value_path_segments(
                    workspace,
                    resolved_module,
                    module_path,
                    scope,
                    &path,
                    span,
                    "memory constructor",
                    diagnostics,
                );
            }
            for attachment in attached {
                validate_header_attachment_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    attachment,
                    diagnostics,
                );
            }
        }
        HirExpr::GenericApply { expr, type_args } => {
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                expr,
                span,
                diagnostics,
            );
            for type_arg in type_args {
                validate_type_surface(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    type_arg,
                    span,
                    &format!("expression generic argument `{type_arg}`"),
                    diagnostics,
                );
            }
        }
        HirExpr::QualifiedPhrase {
            subject,
            args,
            qualifier_kind,
            qualifier,
            qualifier_type_args,
            attached,
            ..
        } => {
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                subject,
                span,
                diagnostics,
            );
            validate_bare_method_resolution(
                workspace,
                resolved_module,
                type_scope,
                scope,
                module_path,
                subject,
                *qualifier_kind,
                qualifier,
                span,
                diagnostics,
            );
            for type_arg in qualifier_type_args {
                validate_type_surface(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    type_arg,
                    span,
                    &format!("phrase qualifier generic argument `{}`", type_arg.render()),
                    diagnostics,
                );
            }
            for arg in args {
                validate_phrase_arg_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    arg,
                    span,
                    diagnostics,
                );
            }
            for attachment in attached {
                validate_header_attachment_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    attachment,
                    diagnostics,
                );
            }
            validate_opaque_constructor_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                subject,
                qualifier_kind,
                qualifier,
                qualifier_type_args,
                args,
                span,
                diagnostics,
            );
        }
        HirExpr::Await { expr } => validate_expr_semantics(
            workspace,
            resolved_module,
            module_path,
            type_scope,
            scope,
            expr,
            span,
            diagnostics,
        ),
        HirExpr::Unary { op, expr } => {
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                expr,
                span,
                diagnostics,
            );
            match op {
                HirUnaryOp::Not => validate_expected_expr_type(
                    module_path,
                    expr,
                    span,
                    diagnostics,
                    ExprTypeClass::Bool,
                    &format!("operand of `{}`", unary_op_token(*op)),
                ),
                HirUnaryOp::Neg | HirUnaryOp::BitNot => validate_expected_expr_type(
                    module_path,
                    expr,
                    span,
                    diagnostics,
                    ExprTypeClass::Int,
                    &format!("operand of `{}`", unary_op_token(*op)),
                ),
                HirUnaryOp::BorrowRead => {
                    validate_borrow_operand_place(
                        workspace,
                        resolved_module,
                        module_path,
                        scope,
                        expr,
                        span,
                        diagnostics,
                        false,
                    );
                }
                HirUnaryOp::BorrowMut => {
                    validate_borrow_operand_place(
                        workspace,
                        resolved_module,
                        module_path,
                        scope,
                        expr,
                        span,
                        diagnostics,
                        true,
                    );
                }
                HirUnaryOp::Deref | HirUnaryOp::Weave | HirUnaryOp::Split => {}
            }
        }
        HirExpr::Binary { left, op, right } => {
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                left,
                span,
                diagnostics,
            );
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                right,
                span,
                diagnostics,
            );
            match op {
                HirBinaryOp::And | HirBinaryOp::Or => {
                    validate_expected_expr_type(
                        module_path,
                        left,
                        span,
                        diagnostics,
                        ExprTypeClass::Bool,
                        &format!("left operand of `{}`", binary_op_token(*op)),
                    );
                    validate_expected_expr_type(
                        module_path,
                        right,
                        span,
                        diagnostics,
                        ExprTypeClass::Bool,
                        &format!("right operand of `{}`", binary_op_token(*op)),
                    );
                }
                HirBinaryOp::Sub
                | HirBinaryOp::Mul
                | HirBinaryOp::Div
                | HirBinaryOp::Mod
                | HirBinaryOp::BitOr
                | HirBinaryOp::BitXor
                | HirBinaryOp::BitAnd
                | HirBinaryOp::Shl
                | HirBinaryOp::Shr => {
                    validate_expected_expr_type(
                        module_path,
                        left,
                        span,
                        diagnostics,
                        ExprTypeClass::Int,
                        &format!("left operand of `{}`", binary_op_token(*op)),
                    );
                    validate_expected_expr_type(
                        module_path,
                        right,
                        span,
                        diagnostics,
                        ExprTypeClass::Int,
                        &format!("right operand of `{}`", binary_op_token(*op)),
                    );
                }
                HirBinaryOp::Add
                | HirBinaryOp::EqEq
                | HirBinaryOp::NotEq
                | HirBinaryOp::Lt
                | HirBinaryOp::LtEq
                | HirBinaryOp::Gt
                | HirBinaryOp::GtEq => {}
            }
        }
        member_expr @ HirExpr::MemberAccess { expr, .. } => {
            if let Some(path) = flatten_member_expr_path(member_expr)
                && should_resolve_member_path_as_namespace(workspace, resolved_module, scope, &path)
            {
                validate_value_path_segments(
                    workspace,
                    resolved_module,
                    module_path,
                    scope,
                    &path,
                    span,
                    "value expression",
                    diagnostics,
                );
                return;
            }
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                expr,
                span,
                diagnostics,
            );
            if let HirExpr::MemberAccess { member, .. } = member_expr
                && is_tuple_projection_member(member)
                && let Some(actual) = infer_expr_type(expr)
                && actual != ExprTypeClass::Pair
            {
                push_type_contract_diagnostic(
                    module_path,
                    span,
                    diagnostics,
                    format!(
                        "tuple field access `.{member}` requires a pair value, found {}",
                        actual.label()
                    ),
                );
            }
        }
        HirExpr::Index { expr, index } => {
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                expr,
                span,
                diagnostics,
            );
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                index,
                span,
                diagnostics,
            );
        }
        HirExpr::Slice {
            expr, start, end, ..
        } => {
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                expr,
                span,
                diagnostics,
            );
            if let Some(start) = start {
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    start,
                    span,
                    diagnostics,
                );
                validate_expected_expr_type(
                    module_path,
                    start,
                    span,
                    diagnostics,
                    ExprTypeClass::Int,
                    "slice start",
                );
            }
            if let Some(end) = end {
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    end,
                    span,
                    diagnostics,
                );
                validate_expected_expr_type(
                    module_path,
                    end,
                    span,
                    diagnostics,
                    ExprTypeClass::Int,
                    "slice end",
                );
            }
        }
        HirExpr::Range { start, end, .. } => {
            if let Some(start) = start {
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    start,
                    span,
                    diagnostics,
                );
            }
            if let Some(end) = end {
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    end,
                    span,
                    diagnostics,
                );
            }
        }
    }
}

fn validate_phrase_arg_semantics(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    type_scope: &TypeScope,
    scope: &ValueScope,
    arg: &arcana_hir::HirPhraseArg,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match arg {
        arcana_hir::HirPhraseArg::Positional(expr)
        | arcana_hir::HirPhraseArg::Named { value: expr, .. } => validate_expr_semantics(
            workspace,
            resolved_module,
            module_path,
            type_scope,
            scope,
            expr,
            span,
            diagnostics,
        ),
    }
}

fn validate_chain_step_semantics(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    type_scope: &TypeScope,
    scope: &ValueScope,
    step: &HirChainStep,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) {
    validate_chain_stage_semantics(
        workspace,
        resolved_module,
        module_path,
        type_scope,
        scope,
        &step.stage,
        span,
        &step.text,
        diagnostics,
    );
    for arg in &step.bind_args {
        validate_expr_semantics(
            workspace,
            resolved_module,
            module_path,
            type_scope,
            scope,
            arg,
            span,
            diagnostics,
        );
    }
}

fn validate_chain_stage_semantics(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    type_scope: &TypeScope,
    scope: &ValueScope,
    stage: &HirExpr,
    span: Span,
    text: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match stage {
        HirExpr::Path { segments } => validate_value_path_segments(
            workspace,
            resolved_module,
            module_path,
            scope,
            segments,
            span,
            &format!("chain step `{text}`"),
            diagnostics,
        ),
        stage @ HirExpr::MemberAccess { .. } => {
            if let Some(path) = flatten_member_expr_path(stage) {
                validate_value_path_segments(
                    workspace,
                    resolved_module,
                    module_path,
                    scope,
                    &path,
                    span,
                    &format!("chain step `{text}`"),
                    diagnostics,
                );
            } else {
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    stage,
                    span,
                    diagnostics,
                );
            }
        }
        HirExpr::GenericApply { expr, type_args } => {
            validate_chain_stage_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                expr,
                span,
                text,
                diagnostics,
            );
            for type_arg in type_args {
                validate_type_surface(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    type_arg,
                    span,
                    &format!("chain step generic argument `{type_arg}` in `{text}`"),
                    diagnostics,
                );
            }
        }
        _ => validate_expr_semantics(
            workspace,
            resolved_module,
            module_path,
            type_scope,
            scope,
            stage,
            span,
            diagnostics,
        ),
    }
}

fn validate_header_attachment_semantics(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    type_scope: &TypeScope,
    scope: &ValueScope,
    attachment: &HirHeaderAttachment,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match attachment {
        HirHeaderAttachment::Named { value, span, .. }
        | HirHeaderAttachment::Chain {
            expr: value, span, ..
        } => validate_expr_semantics(
            workspace,
            resolved_module,
            module_path,
            type_scope,
            scope,
            value,
            *span,
            diagnostics,
        ),
    }
}

fn validate_match_pattern_semantics(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    subject: Option<&HirExpr>,
    pattern: &HirMatchPattern,
    scope: &mut ValueScope,
) {
    match pattern {
        HirMatchPattern::Wildcard | HirMatchPattern::Literal { .. } => {}
        HirMatchPattern::Name { text } => {
            let is_binding = match split_simple_path(text) {
                Some(path) => {
                    path.len() == 1
                        && !subject.is_some_and(|subject| {
                            match_name_resolves_to_zero_payload_variant(
                                workspace,
                                resolved_module,
                                scope,
                                subject,
                                text.trim(),
                            )
                        })
                }
                None => true,
            };
            if is_binding {
                scope.insert(text.trim(), false);
            }
        }
        HirMatchPattern::Variant { args, .. } => {
            for arg in args {
                validate_match_pattern_semantics(workspace, resolved_module, None, arg, scope);
            }
        }
    }
}

fn validate_value_path_segments(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    scope: &ValueScope,
    path: &[String],
    span: Span,
    context: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if path.len() == 1 && scope.contains(&path[0]) {
        return;
    }
    if value_path_exists(workspace, resolved_module, path) {
        return;
    }
    diagnostics.push(Diagnostic {
        path: module_path.to_path_buf(),
        line: span.line,
        column: span.column,
        message: format!(
            "unresolved value reference `{}` in {context}",
            path.join(".")
        ),
    });
}

fn value_path_exists(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    path: &[String],
) -> bool {
    if path.is_empty() {
        return false;
    }
    if let Some(binding) = resolved_module.bindings.get(&path[0])
        && target_path_exists(workspace, &binding.target, &path[1..])
    {
        return true;
    }
    let Some(package) = visible_package_root_for_module(workspace, resolved_module, &path[0])
    else {
        let Some(package) = current_workspace_package_for_module(workspace, resolved_module) else {
            return false;
        };
        return package_path_exists(package, path);
    };
    package_path_exists(package, &path[1..])
}

fn target_path_exists(
    workspace: &HirWorkspaceSummary,
    target: &HirResolvedTarget,
    tail: &[String],
) -> bool {
    match target {
        HirResolvedTarget::Symbol {
            package_id,
            module_id,
            symbol_name,
            ..
        } => {
            let Some(package) = workspace.package_by_id(package_id) else {
                return false;
            };
            let Some(module) = package.module(module_id) else {
                return false;
            };
            let Some(symbol) = module
                .symbols
                .iter()
                .find(|symbol| symbol.name == *symbol_name)
            else {
                return false;
            };
            symbol_tail_exists(symbol, tail)
        }
        HirResolvedTarget::Module {
            package_id,
            module_id,
            ..
        } => {
            let Some(package) = workspace.package_by_id(package_id) else {
                return false;
            };
            let Some(module) = package.module(module_id) else {
                return false;
            };
            module_path_exists(package, module, tail)
        }
    }
}

fn package_path_exists(package: &HirWorkspacePackage, path: &[String]) -> bool {
    if path.is_empty() {
        return true;
    }
    let Some(module) = package.module(&package.summary.package_name) else {
        return false;
    };
    module_path_exists(package, module, path)
}

fn module_path_exists(
    package: &HirWorkspacePackage,
    module: &HirModuleSummary,
    path: &[String],
) -> bool {
    if path.is_empty() {
        return true;
    }
    if module_value_member_exists(module, &path[0], &path[1..]) {
        return true;
    }
    let base_relative = module
        .module_id
        .split('.')
        .skip(1)
        .map(str::to_string)
        .collect::<Vec<_>>();
    let mut module_candidate = base_relative.clone();
    module_candidate.extend_from_slice(path);
    if package.resolve_relative_module(&module_candidate).is_some() {
        return true;
    }
    for split_index in 1..path.len() {
        let mut symbol_module_path = base_relative.clone();
        symbol_module_path.extend_from_slice(&path[..split_index]);
        let Some(target_module) = package.resolve_relative_module(&symbol_module_path) else {
            continue;
        };
        if module_value_member_exists(target_module, &path[split_index], &path[split_index + 1..]) {
            return true;
        }
    }
    false
}

fn module_value_member_exists(module: &HirModuleSummary, member: &str, tail: &[String]) -> bool {
    if let Some(symbol) = module.symbols.iter().find(|symbol| symbol.name == member)
        && symbol_tail_exists(symbol, tail)
    {
        return true;
    }
    tail.is_empty()
        && module
            .impls
            .iter()
            .flat_map(|impl_decl| impl_decl.methods.iter())
            .any(|method| method.name == member)
}

fn symbol_tail_exists(symbol: &HirSymbol, tail: &[String]) -> bool {
    if tail.is_empty() {
        return true;
    }
    match &symbol.body {
        HirSymbolBody::Enum { variants } => {
            tail.len() == 1 && variants.iter().any(|variant| variant.name == tail[0])
        }
        _ => false,
    }
}

fn should_resolve_member_path_as_namespace(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    scope: &ValueScope,
    path: &[String],
) -> bool {
    if path.len() < 2 || scope.contains(&path[0]) {
        return false;
    }
    if let Some(binding) = resolved_module.bindings.get(&path[0]) {
        return match &binding.target {
            HirResolvedTarget::Module { .. } => true,
            HirResolvedTarget::Symbol { .. } => {
                target_supports_member_namespace(workspace, &binding.target)
            }
        };
    }
    if visible_package_root_for_module(workspace, resolved_module, &path[0]).is_some() {
        return true;
    }
    let Some(package) = current_workspace_package_for_module(workspace, resolved_module) else {
        return false;
    };
    package
        .resolve_relative_module(&[path[0].clone()])
        .is_some()
}

fn target_supports_member_namespace(
    workspace: &HirWorkspaceSummary,
    target: &HirResolvedTarget,
) -> bool {
    match target {
        HirResolvedTarget::Module { .. } => true,
        HirResolvedTarget::Symbol {
            package_id,
            module_id,
            symbol_name,
            ..
        } => workspace
            .package_by_id(package_id)
            .and_then(|package| package.module(module_id))
            .and_then(|module| {
                module
                    .symbols
                    .iter()
                    .find(|symbol| symbol.name == *symbol_name)
            })
            .map(|symbol| matches!(symbol.body, HirSymbolBody::Enum { .. }))
            .unwrap_or(false),
    }
}

fn flatten_member_expr_path(expr: &HirExpr) -> Option<Vec<String>> {
    match expr {
        HirExpr::Path { segments } => Some(segments.clone()),
        HirExpr::MemberAccess { expr, member } if is_identifier_text(member) => {
            let mut path = flatten_member_expr_path(expr)?;
            path.push(member.clone());
            Some(path)
        }
        _ => None,
    }
}

fn flatten_assign_target_path(target: &HirAssignTarget) -> Option<Vec<String>> {
    match target {
        HirAssignTarget::Name { text } => Some(vec![text.clone()]),
        HirAssignTarget::MemberAccess { target, member } if is_identifier_text(member) => {
            let mut path = flatten_assign_target_path(target)?;
            path.push(member.clone());
            Some(path)
        }
        _ => None,
    }
}

fn is_identifier_text(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum BindingPattern {
    Name(String),
    Pair(Box<BindingPattern>, Box<BindingPattern>),
}

fn split_binding_pattern_top_level(source: &str, separator: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (idx, ch) in source.char_indices() {
        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            _ if ch == separator && depth == 0 => {
                parts.push(source[start..idx].trim());
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(source[start..].trim());
    parts
}

fn parse_binding_pattern(text: &str) -> Option<BindingPattern> {
    let trimmed = text.trim();
    if is_identifier_text(trimmed) {
        return Some(BindingPattern::Name(trimmed.to_string()));
    }
    let inner = trimmed.strip_prefix('(')?.strip_suffix(')')?;
    let parts = split_binding_pattern_top_level(inner, ',');
    if parts.len() != 2 {
        return None;
    }
    Some(BindingPattern::Pair(
        Box::new(parse_binding_pattern(parts[0])?),
        Box::new(parse_binding_pattern(parts[1])?),
    ))
}

fn binding_pattern_is_destructuring(text: &str) -> bool {
    matches!(parse_binding_pattern(text), Some(BindingPattern::Pair(_, _)))
}

fn collect_binding_pattern_names(pattern: &BindingPattern, names: &mut Vec<String>) {
    match pattern {
        BindingPattern::Name(name) => names.push(name.clone()),
        BindingPattern::Pair(left, right) => {
            collect_binding_pattern_names(left, names);
            collect_binding_pattern_names(right, names);
        }
    }
}

fn collect_typed_binding_pattern_entries(
    pattern: &BindingPattern,
    ty: Option<&HirType>,
    entries: &mut Vec<(String, Option<HirType>)>,
) -> Result<(), String> {
    match pattern {
        BindingPattern::Name(name) => {
            entries.push((name.clone(), ty.cloned()));
            Ok(())
        }
        BindingPattern::Pair(left, right) => {
            let Some(ty) = ty else {
                collect_typed_binding_pattern_entries(left, None, entries)?;
                return collect_typed_binding_pattern_entries(right, None, entries);
            };
            let HirTypeKind::Tuple(items) = &ty.kind else {
                return Err(format!(
                    "tuple destructuring requires a pair value, found {}",
                    ty.render()
                ));
            };
            let [left_ty, right_ty] = items.as_slice() else {
                return Err(format!(
                    "tuple destructuring requires a pair value, found {}",
                    ty.render()
                ));
            };
            collect_typed_binding_pattern_entries(left, Some(left_ty), entries)?;
            collect_typed_binding_pattern_entries(right, Some(right_ty), entries)
        }
    }
}

fn bind_pattern_into_scope(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    type_scope: &TypeScope,
    scope: &mut ValueScope,
    pattern_text: &str,
    mutable: bool,
    value_ty: Option<HirType>,
) -> Result<Vec<String>, String> {
    let pattern = parse_binding_pattern(pattern_text)
        .ok_or_else(|| format!("invalid binding pattern `{pattern_text}`"))?;
    let mut entries = Vec::new();
    collect_typed_binding_pattern_entries(&pattern, value_ty.as_ref(), &mut entries)?;
    let mut seen = BTreeSet::new();
    let mut inserted = Vec::new();
    for (name, ty) in entries {
        if !seen.insert(name.clone()) {
            return Err(format!("duplicate binding `{name}` in tuple pattern"));
        }
        let ownership = ty
            .as_ref()
            .map(|ty| infer_type_ownership(workspace, resolved_module, type_scope, ty))
            .unwrap_or_default();
        scope.insert_typed(&name, mutable, ownership, ty);
        inserted.push(name);
    }
    Ok(inserted)
}

#[cfg(test)]
mod tests {
    use super::{
        AdapterArtifactIdentity, AdapterCatalogEntry, AdapterForewordSnapshot,
        AdapterPackageSnapshot, AdapterTargetSnapshot, BUILTIN_FOREWORD_PROVIDER_PACKAGE_ID,
        FOREWORD_ADAPTER_PROTOCOL_VERSION, ForewordAdapterRequest, ResolvedForewordExport,
        ResolvedForewordExportKind, build_adapter_artifact_identity,
        build_foreword_adapter_cache_key, check_path, check_sources, check_workspace_graph,
        compute_member_fingerprints, compute_member_fingerprints_for_checked_workspace,
        execute_executable_foreword_app, execute_foreword_adapter, load_workspace_hir,
        lower_adapter_payload_args, lower_to_hir, materialize_foreword_adapter_artifact,
    };
    use arcana_package::{
        BuildDisposition, execute_build, load_workspace_graph, plan_workspace, prepare_build,
        read_lockfile, write_lockfile,
    };
    use arcana_syntax::Span;
    use std::collections::{BTreeMap, BTreeSet};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT_TEST_ID: AtomicU64 = AtomicU64::new(1);

    fn plan_build(
        graph: &arcana_package::WorkspaceGraph,
        order: &[String],
        _fingerprints: &arcana_package::WorkspaceFingerprints,
        existing_lock: Option<&arcana_package::Lockfile>,
    ) -> Result<Vec<arcana_package::BuildStatus>, String> {
        let prepared = prepare_build(graph)?;
        arcana_package::plan_build(graph, order, &prepared, existing_lock)
    }

    fn execute_planned_build(
        graph: &arcana_package::WorkspaceGraph,
        _fingerprints: &arcana_package::WorkspaceFingerprints,
        statuses: &[arcana_package::BuildStatus],
    ) {
        let prepared = prepare_build(graph).expect("prepare build");
        execute_build(graph, &prepared, statuses).expect("execute");
    }

    #[test]
    fn check_sources_counts_modules() {
        let summary = check_sources(
            ["import std.io\nfn main() -> Int:\n    return 0\n"]
                .iter()
                .copied(),
        )
        .expect("check should pass");
        assert_eq!(summary.module_count, 1);
        assert_eq!(summary.directive_count, 1);
        assert!(summary.symbol_count >= 1);

        let hir = lower_to_hir(&summary);
        assert!(hir.symbol_count >= 1);
    }

    #[test]
    fn check_path_reports_unresolved_import() {
        let root = make_temp_package(
            "broken_app",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "import missing.module\nfn main() -> Int:\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("expected unresolved import");
        assert!(err.contains("missing.module"), "{err}");

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_accepts_workspace_root_package_with_members_and_deps() {
        let root = make_temp_workspace(
            "workspace_root_package",
            &["app"],
            &[
                (
                    "src/shelf.arc",
                    "import core\nfn main() -> Int:\n    return core.value :: :: call\n",
                ),
                ("src/types.arc", ""),
                ("app/book.toml", "name = \"app\"\nkind = \"app\"\n"),
                ("app/src/shelf.arc", "fn main() -> Int:\n    return 0\n"),
                ("app/src/types.arc", ""),
                ("core/book.toml", "name = \"core\"\nkind = \"lib\"\n"),
                (
                    "core/src/book.arc",
                    "export fn value() -> Int:\n    return 7\n",
                ),
                ("core/src/types.arc", ""),
            ],
        );
        fs::write(
            root.join("book.toml"),
            concat!(
                "name = \"workspace\"\n",
                "kind = \"app\"\n",
                "[workspace]\n",
                "members = [\"app\"]\n",
                "[deps]\n",
                "core = { path = \"core\" }\n",
            ),
        )
        .expect("workspace manifest should be writable");

        let summary = check_path(&root).expect("workspace root package should check");
        assert_eq!(summary.package_count, 3);
        assert_eq!(summary.module_count, 6);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_accepts_dependency_alias_imports() {
        let root = make_temp_workspace(
            "dependency_alias_imports",
            &["app", "core"],
            &[
                (
                    "app/book.toml",
                    "name = \"app\"\nkind = \"app\"\n\n[deps]\nutil = { path = \"../core\" }\n",
                ),
                (
                    "app/src/shelf.arc",
                    "import util\nfn main() -> Int:\n    return util.value :: :: call\n",
                ),
                ("app/src/types.arc", ""),
                ("core/book.toml", "name = \"core\"\nkind = \"lib\"\n"),
                (
                    "core/src/book.arc",
                    "export fn value() -> Int:\n    return 7\n",
                ),
                ("core/src/types.arc", ""),
            ],
        );

        let summary = check_path(&root.join("app")).expect("dependency alias import should check");
        assert_eq!(summary.package_count, 2);
        assert!(summary.module_count >= 4);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_accepts_workspace_root_dependency_alias_imports() {
        let root = make_temp_workspace(
            "workspace_root_dependency_aliases",
            &["app"],
            &[
                (
                    "src/shelf.arc",
                    "import util\nfn main() -> Int:\n    return util.value :: :: call\n",
                ),
                ("src/types.arc", ""),
                ("app/book.toml", "name = \"app\"\nkind = \"app\"\n"),
                ("app/src/shelf.arc", "fn main() -> Int:\n    return 0\n"),
                ("app/src/types.arc", ""),
                ("core/book.toml", "name = \"core\"\nkind = \"lib\"\n"),
                (
                    "core/src/book.arc",
                    "export fn value() -> Int:\n    return 7\n",
                ),
                ("core/src/types.arc", ""),
            ],
        );
        fs::write(
            root.join("book.toml"),
            concat!(
                "name = \"workspace\"\n",
                "kind = \"app\"\n",
                "[workspace]\n",
                "members = [\"app\"]\n",
                "[deps]\n",
                "util = { path = \"core\" }\n",
            ),
        )
        .expect("workspace manifest should be writable");

        let summary =
            check_path(&root).expect("workspace root dependency alias import should check");
        assert_eq!(summary.package_count, 3);
        assert_eq!(summary.module_count, 6);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_workspace_graph_allows_local_modules_named_like_unrelated_members() {
        let root = make_temp_workspace(
            "workspace_local_module_shadowing_member",
            &["app", "core"],
            &[
                ("app/book.toml", "name = \"app\"\nkind = \"app\"\n"),
                (
                    "app/src/shelf.arc",
                    "import core\nfn main() -> Int:\n    return core.value :: :: call\n",
                ),
                (
                    "app/src/core.arc",
                    "export fn value() -> Int:\n    return 7\n",
                ),
                ("app/src/types.arc", ""),
                ("core/book.toml", "name = \"core\"\nkind = \"lib\"\n"),
                (
                    "core/src/book.arc",
                    "export fn value() -> Int:\n    return 0\n",
                ),
                ("core/src/types.arc", ""),
            ],
        );

        let graph = load_workspace_graph(&root).expect("workspace graph should load");
        let summary = check_workspace_graph(&graph)
            .expect("workspace check should prefer app-local modules over unrelated members");
        assert_eq!(summary.summary().package_count, 2);
        assert!(summary.summary().module_count >= 5);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_main_with_parameters() {
        let root = make_temp_package(
            "main_with_parameters",
            "app",
            &[],
            &[
                ("src/shelf.arc", "fn main(x: Int) -> Int:\n    return x\n"),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("parameterized main should fail");
        assert!(
            err.contains("main must not take parameters in the current runtime lane"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_main_with_non_runtime_return_type() {
        let root = make_temp_package(
            "main_with_bool_return",
            "app",
            &[],
            &[
                ("src/shelf.arc", "fn main() -> Bool:\n    return true\n"),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("non-runtime main return should fail");
        assert!(
            err.contains("main must return Int or Unit in the current runtime lane"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_sources_rejects_tuple_contract_fixtures() {
        let repo_root = repo_root();
        for (fixture, expected) in [
            (
                "tuple_field_out_of_range.arc",
                "tuple field access only supports `.0` and `.1` in v1",
            ),
            (
                "tuple_triple_type.arc",
                "tuple types must have exactly 2 elements in v1",
            ),
            (
                "tuple_field_assignment.arc",
                "tuple field assignment is not allowed in v1",
            ),
        ] {
            let source = fs::read_to_string(
                repo_root
                    .join("conformance")
                    .join("check_parity_fixtures")
                    .join(fixture),
            )
            .expect("fixture should be readable");
            let err = match check_sources([source.as_str()]) {
                Ok(summary) => panic!("{fixture} unexpectedly passed: {summary:?}"),
                Err(err) => err,
            };
            assert!(err.contains(expected), "{fixture}: {err}");
        }
    }

    #[test]
    fn check_sources_accept_tuple_destructuring_in_let_and_for() {
        let summary = check_sources([concat!(
            "fn main() -> Int:\n",
            "    let pair = (1, 2)\n",
            "    let (left, right) = pair\n",
            "    for (first, second) in [(left, right)]:\n",
            "        return first + second\n",
            "    return 0\n",
        )])
        .expect("tuple destructuring should check");
        assert_eq!(summary.module_count, 1);
    }

    #[test]
    fn check_sources_accept_owner_activation_with_explicit_context_clause() {
        let root = make_temp_package(
            "owner_explicit_context_positive",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    concat!(
                        "obj SessionCtx:\n",
                        "    base: Int\n",
                        "\n",
                        "obj Counter:\n",
                        "    value: Int\n",
                        "    fn init(edit self: Self, read ctx: SessionCtx):\n",
                        "        self.value = ctx.base\n",
                        "\n",
                        "create Session [Counter] context: SessionCtx scope-exit:\n",
                        "    done: when Counter.value > 10 hold [Counter]\n",
                        "\n",
                        "Session\n",
                        "Counter\n",
                        "fn main() -> Int:\n",
                        "    let ctx = SessionCtx :: base = 12 :: call\n",
                        "    Session :: ctx :: call\n",
                        "    return Counter.value\n",
                    ),
                ),
                ("src/types.arc", ""),
            ],
        );

        let summary = check_path(&root).expect("owner with explicit context should check");
        assert_eq!(summary.package_count, 1);
        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_sources_accept_must_and_fallback_for_option_and_result() {
        let summary = check_sources([concat!(
            "enum Option[T]:\n",
            "    Some(T)\n",
            "    None\n",
            "\n",
            "enum Result[T, E]:\n",
            "    Ok(T)\n",
            "    Err(E)\n",
            "\n",
            "fn main() -> Int:\n",
            "    let maybe = Option.Some[Int] :: 4 :: call\n",
            "    let none_value = Option.None[Int] :: :: call\n",
            "    let fallback = none_value :: 7 :: fallback\n",
            "    let ok = Result.Ok[Int, Str] :: 3 :: call\n",
            "    let maybe_value = maybe :: :: must\n",
            "    let must_value = ok :: :: must\n",
            "    return maybe_value + fallback + must_value\n",
        )])
        .expect("must/fallback surface should type-check");
        assert_eq!(summary.module_count, 1);
    }

    #[test]
    fn check_sources_rejects_headed_region_contract_fixtures() {
        let repo_root = repo_root();
        for (fixture, expected) in [
            (
                "headed_regions_unknown_head.arc",
                "unknown headed region head `recyclez`",
            ),
            (
                "headed_regions_invalid_construct.arc",
                "`construct yield` is expression-form only",
            ),
            (
                "headed_regions_invalid_memory_key.arc",
                "invalid `Memory` detail key `unknown`",
            ),
            (
                "headed_regions_invalid_memory_family.arc",
                "invalid `Memory` family `view`",
            ),
            (
                "headed_regions_unsupported_memory_family_key.arc",
                "memory detail `recycle` is not supported for family `arena`",
            ),
            (
                "headed_regions_missing_modifier.arc",
                "recycle requires a default modifier in v1",
            ),
            (
                "headed_regions_invalid_owner_exit.arc",
                "named recycle exit `done` is not active on this path",
            ),
            (
                "headed_regions_invalid_recycle_return.arc",
                "bare `-return` in recycle requires Result failure",
            ),
            (
                "headed_regions_invalid_bind_modifier.arc",
                "`bind require` only supports `return`, `break`, or `continue` failure handling",
            ),
            (
                "headed_regions_bind_preserve_on_let.arc",
                "`bind -preserve` is only valid on `name = gate` lines",
            ),
            (
                "headed_regions_bind_preserve_payload_type.arc",
                "`bind -preserve` payload for `value` must have type `Str`",
            ),
            (
                "headed_regions_bind_replace_payload_type.arc",
                "`bind -replace` payload for `value` must have type `Str`",
            ),
            (
                "headed_regions_invalid_recycle_continue.arc",
                "`break` and `continue` recycle exits are only valid inside loops",
            ),
            (
                "headed_regions_invalid_construct_skip.arc",
                "construct `-skip` is only valid for Option fields",
            ),
            (
                "headed_regions_invalid_construct_place_type.arc",
                "construct place target type",
            ),
            (
                "headed_regions_construct_duplicate_field.arc",
                "construct field `value` is provided more than once",
            ),
            (
                "headed_regions_construct_duplicate_payload.arc",
                "requires exactly one `payload = ...` line",
            ),
            (
                "headed_regions_invalid_nested.arc",
                "headed regions cannot nest inside another headed region in v1",
            ),
        ] {
            let source = fs::read_to_string(
                repo_root
                    .join("conformance")
                    .join("check_parity_fixtures")
                    .join(fixture),
            )
            .expect("fixture should be readable");
            let root = make_temp_package(
                fixture.trim_end_matches(".arc"),
                "app",
                &[],
                &[("src/shelf.arc", source.as_str()), ("src/types.arc", "")],
            );
            let err = check_path(&root).expect_err("fixture should fail");
            assert!(err.contains(expected), "{fixture}: {err}");
            fs::remove_dir_all(root).expect("cleanup should succeed");
        }
    }

    #[test]
    fn check_headed_region_positive_workspace_fixture_passes() {
        let fixture_root = repo_root()
            .join("conformance")
            .join("fixtures")
            .join("headed_regions_v1_workspace");
        let workspace_book = fs::read_to_string(fixture_root.join("book.toml"))
            .expect("positive headed region workspace manifest should be readable");
        let app_book = fs::read_to_string(fixture_root.join("app").join("book.toml"))
            .expect("positive headed region app manifest should be readable");
        let shelf = fs::read_to_string(fixture_root.join("app").join("src").join("shelf.arc"))
            .expect("positive headed region shelf should be readable");
        let core_book = fs::read_to_string(fixture_root.join("core").join("book.toml"))
            .expect("positive headed region core manifest should be readable");
        let core_module =
            fs::read_to_string(fixture_root.join("core").join("src").join("book.arc"))
                .expect("positive headed region core book should be readable");
        let core_types =
            fs::read_to_string(fixture_root.join("core").join("src").join("types.arc"))
                .expect("positive headed region core types should be readable");
        let root = make_temp_workspace(
            "headed_region_positive_fixture",
            &["core", "app"],
            &[
                ("book.toml", &workspace_book),
                ("app/book.toml", &app_book),
                ("app/src/shelf.arc", &shelf),
                ("app/src/types.arc", ""),
                ("core/book.toml", &core_book),
                ("core/src/book.arc", &core_module),
                ("core/src/types.arc", &core_types),
            ],
        );
        check_path(&root).expect("headed region positive fixture should pass");
        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_sources_rejects_cleanup_footer_contract_fixtures() {
        let repo_root = repo_root();
        for (fixture, expected) in [
            (
                "cleanup_footer_stray.arc",
                "cleanup footer without a valid owning header",
            ),
            (
                "cleanup_footer_bad_target.arc",
                "cleanup footer target must be a binding name",
            ),
        ] {
            let source = fs::read_to_string(
                repo_root
                    .join("conformance")
                    .join("check_parity_fixtures")
                    .join(fixture),
            )
            .expect("fixture should be readable");
            let err = check_sources([source.as_str()]).expect_err("fixture should fail");
            assert!(err.contains(expected), "{fixture}: {err}");
        }
    }

    #[test]
    fn check_sources_rejects_foreword_and_intrinsic_contract_fixtures() {
        let repo_root = repo_root();
        for (fixture, expected) in [
            (
                "invalid_statement_foreword.arc",
                "`#inline` is not a valid statement-level contract",
            ),
            (
                "phrase_arg_arity.arc",
                "qualified phrase allows at most 3 top-level arguments",
            ),
            (
                "phrase_arg_shape.arc",
                "trailing comma is not allowed before phrase qualifier",
            ),
            (
                "memory_phrase_arg_arity.arc",
                "memory phrase allows at most 3 top-level arguments",
            ),
            (
                "unknown_memory_type.arc",
                "unknown memory type `weird`; supported now: arena, frame, pool (reserved for future expansion)",
            ),
            (
                "unknown_chain_style.arc",
                "unknown chain style `mystery`; supported: forward, lazy, parallel, async, plan, broadcast, collect",
            ),
            (
                "reverse_parallel_chain.arc",
                "chain style `parallel` does not support reverse-introduced chains",
            ),
            (
                "unknown_memory_type.arc",
                "unknown memory type `weird`; supported now: arena, frame, pool (reserved for future expansion)",
            ),
            (
                "invalid_boundary_payload.arc",
                "invalid payload for foreword `#boundary`: `target` must be a string or symbol",
            ),
            (
                "test_payload.arc",
                "invalid payload for foreword `#test`: expected no payload",
            ),
            (
                "invalid_stage_contract_key.arc",
                "invalid #stage contract key 'bad_key'",
            ),
            (
                "invalid_chain_contract_key.arc",
                "invalid #chain contract key 'bad_key'",
            ),
            (
                "invalid_chain_contract_phase.arc",
                "invalid payload for `phase`",
            ),
            (
                "malformed_intrinsic.arc",
                "malformed intrinsic function declaration",
            ),
        ] {
            let source = fs::read_to_string(
                repo_root
                    .join("conformance")
                    .join("check_parity_fixtures")
                    .join(fixture),
            )
            .expect("fixture should be readable");
            let err = check_sources([source.as_str()]).expect_err("fixture should fail");
            assert!(err.contains(expected), "{fixture}: {err}");
        }
    }

    #[test]
    fn check_path_rejects_unresolved_tuple_value_package() {
        let err = check_path(
            &repo_root()
                .join("conformance")
                .join("check_parity_packages")
                .join("unresolved_tuple_value_ref"),
        )
        .expect_err("fixture should fail");
        assert!(
            err.contains("unresolved value reference `missing` in value expression"),
            "{err}"
        );
    }

    #[test]
    fn check_path_resolves_local_use_symbols() {
        let root = make_temp_package(
            "counter_app",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "import types\nuse types.Counter\nfn main() -> Int:\n    return 0\n",
                ),
                ("src/types.arc", "export record Counter:\n    value: Int\n"),
            ],
        );

        let summary = check_path(&root).expect("local symbols should resolve");
        assert_eq!(summary.module_count, 2);
        assert_eq!(summary.package_count, 1);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn typed_api_fingerprint_ignores_equivalent_export_type_spelling() {
        let root = make_temp_workspace(
            "typed_api_surface_spelling",
            &["app", "core"],
            &[
                (
                    "app/book.toml",
                    "name = \"app\"\nkind = \"app\"\n\n[deps]\ncore = { path = \"../core\" }\n",
                ),
                (
                    "app/src/shelf.arc",
                    "import core\nfn main() -> Int:\n    let value = core.make_counter :: :: call\n    return value.value\n",
                ),
                ("app/src/types.arc", ""),
                ("core/book.toml", "name = \"core\"\nkind = \"lib\"\n"),
                (
                    "core/src/book.arc",
                    "import types\nuse types.Counter\nexport fn make_counter() -> Counter:\n    return Counter :: value = 0 :: call\n",
                ),
                (
                    "core/src/types.arc",
                    "export record Counter:\n    value: Int\n",
                ),
            ],
        );

        let graph = load_workspace_graph(&root).expect("load graph");
        let core_id = graph
            .member("core")
            .expect("core member should resolve")
            .package_id
            .clone();
        let app_id = graph
            .member("app")
            .expect("app member should resolve")
            .package_id
            .clone();
        let order = plan_workspace(&graph).expect("plan");
        let first_fingerprints = compute_member_fingerprints(&graph).expect("fingerprints");
        let first_statuses = plan_build(&graph, &order, &first_fingerprints, None).expect("plan");
        execute_planned_build(&graph, &first_fingerprints, &first_statuses);
        let lock_path = write_lockfile(&graph, &order, &first_statuses).expect("lock");
        let existing = read_lockfile(&lock_path).expect("read").expect("lock");

        fs::write(
            root.join("core/src/book.arc"),
            "import types\nexport fn make_counter() -> types.Counter:\n    return types.Counter :: value = 0 :: call\n",
        )
        .expect("rewrite should succeed");

        let second_fingerprints = compute_member_fingerprints(&graph).expect("fingerprints");
        let second_statuses =
            plan_build(&graph, &order, &second_fingerprints, Some(&existing)).expect("plan");
        assert_eq!(second_statuses[0].member(), core_id);
        assert_eq!(second_statuses[0].disposition(), BuildDisposition::Built);
        assert_eq!(second_statuses[1].member(), app_id);
        assert_eq!(second_statuses[1].disposition(), BuildDisposition::Built);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn compute_member_fingerprints_reuses_checked_workspace_state() {
        let root = make_temp_workspace(
            "typed_api_checked_workspace",
            &["app", "core"],
            &[
                (
                    "app/book.toml",
                    "name = \"app\"\nkind = \"app\"\n\n[deps]\ncore = { path = \"../core\" }\n",
                ),
                (
                    "app/src/shelf.arc",
                    "import core\nfn main() -> Int:\n    return core.value :: :: call\n",
                ),
                ("app/src/types.arc", ""),
                ("core/book.toml", "name = \"core\"\nkind = \"lib\"\n"),
                (
                    "core/src/book.arc",
                    "export fn value() -> Int:\n    return 0\n",
                ),
                ("core/src/types.arc", ""),
            ],
        );

        let graph = load_workspace_graph(&root).expect("load graph");
        let checked = check_workspace_graph(&graph).expect("workspace should check");
        let direct = compute_member_fingerprints(&graph).expect("direct fingerprints");
        let reused = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
            .expect("reused fingerprints");
        assert_eq!(direct, reused);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn member_source_fingerprint_ignores_whitespace_only_edits() {
        let root = make_temp_workspace(
            "typed_api_whitespace_source",
            &["app"],
            &[
                ("app/book.toml", "name = \"app\"\nkind = \"app\"\n"),
                ("app/src/shelf.arc", "fn main() -> Int:\n    return 0\n"),
                ("app/src/types.arc", ""),
            ],
        );

        let graph = load_workspace_graph(&root).expect("load graph");
        let order = plan_workspace(&graph).expect("plan");
        let first_fingerprints = compute_member_fingerprints(&graph).expect("fingerprints");
        let first_statuses = plan_build(&graph, &order, &first_fingerprints, None).expect("plan");
        execute_planned_build(&graph, &first_fingerprints, &first_statuses);
        let lock_path = write_lockfile(&graph, &order, &first_statuses).expect("lock");
        let existing = read_lockfile(&lock_path).expect("read").expect("lock");

        fs::write(
            root.join("app/src/shelf.arc"),
            "fn main() -> Int:\n\n    return 0\n",
        )
        .expect("rewrite should succeed");

        let second_fingerprints = compute_member_fingerprints(&graph).expect("fingerprints");
        assert_eq!(
            first_fingerprints
                .get("app")
                .map(|fingerprint| &fingerprint.source),
            second_fingerprints
                .get("app")
                .map(|fingerprint| &fingerprint.source)
        );
        assert_eq!(
            first_fingerprints
                .get("app")
                .map(|fingerprint| &fingerprint.api),
            second_fingerprints
                .get("app")
                .map(|fingerprint| &fingerprint.api)
        );

        let second_statuses =
            plan_build(&graph, &order, &second_fingerprints, Some(&existing)).expect("plan");
        assert!(
            second_statuses
                .iter()
                .all(|status| status.disposition() == BuildDisposition::CacheHit)
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn typed_api_fingerprint_ignores_private_dependency_code_edits() {
        let root = make_temp_workspace(
            "typed_api_private_dependency_code",
            &["app", "core"],
            &[
                (
                    "app/book.toml",
                    "name = \"app\"\nkind = \"app\"\n\n[deps]\ncore = { path = \"../core\" }\n",
                ),
                (
                    "app/src/shelf.arc",
                    "import core\nfn main() -> Int:\n    return core.shared_value :: :: call\n",
                ),
                ("app/src/types.arc", ""),
                ("core/book.toml", "name = \"core\"\nkind = \"lib\"\n"),
                (
                    "core/src/book.arc",
                    "export fn shared_value() -> Int:\n    return helper :: :: call\n",
                ),
                ("core/src/helper.arc", "fn helper() -> Int:\n    return 0\n"),
                ("core/src/types.arc", ""),
            ],
        );

        let graph = load_workspace_graph(&root).expect("load graph");
        let core_id = graph
            .member("core")
            .expect("core member should resolve")
            .package_id
            .clone();
        let app_id = graph
            .member("app")
            .expect("app member should resolve")
            .package_id
            .clone();
        let order = plan_workspace(&graph).expect("plan");
        let first_fingerprints = compute_member_fingerprints(&graph).expect("fingerprints");
        let first_statuses = plan_build(&graph, &order, &first_fingerprints, None).expect("plan");
        execute_planned_build(&graph, &first_fingerprints, &first_statuses);
        let lock_path = write_lockfile(&graph, &order, &first_statuses).expect("lock");
        let existing = read_lockfile(&lock_path).expect("read").expect("lock");

        fs::write(
            root.join("core/src/helper.arc"),
            "fn helper() -> Int:\n    return 1\n",
        )
        .expect("rewrite should succeed");

        let second_fingerprints = compute_member_fingerprints(&graph).expect("fingerprints");
        assert_ne!(
            first_fingerprints
                .get(&core_id)
                .map(|fingerprint| &fingerprint.source),
            second_fingerprints
                .get(&core_id)
                .map(|fingerprint| &fingerprint.source)
        );
        assert_eq!(
            first_fingerprints
                .get(&core_id)
                .map(|fingerprint| &fingerprint.api),
            second_fingerprints
                .get(&core_id)
                .map(|fingerprint| &fingerprint.api)
        );

        let second_statuses =
            plan_build(&graph, &order, &second_fingerprints, Some(&existing)).expect("plan");
        assert_eq!(second_statuses[0].member(), core_id);
        assert_eq!(second_statuses[0].disposition(), BuildDisposition::Built);
        assert_eq!(second_statuses[1].member(), app_id);
        assert_eq!(second_statuses[1].disposition(), BuildDisposition::Built);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_workspace_rejects_private_dependency_symbol_use() {
        let root = make_temp_workspace(
            "private_dependency_symbol_use",
            &["app", "core"],
            &[
                (
                    "app/book.toml",
                    "name = \"app\"\nkind = \"app\"\n\n[deps]\ncore = { path = \"../core\" }\n",
                ),
                (
                    "app/src/shelf.arc",
                    "use core.hidden\nfn main() -> Int:\n    return hidden :: :: call\n",
                ),
                ("app/src/types.arc", ""),
                ("core/book.toml", "name = \"core\"\nkind = \"lib\"\n"),
                (
                    "core/src/book.arc",
                    "export fn shared() -> Int:\n    return 1\nfn hidden() -> Int:\n    return 0\n",
                ),
                ("core/src/types.arc", ""),
            ],
        );

        let graph = load_workspace_graph(&root).expect("load graph");
        let err = match check_workspace_graph(&graph) {
            Ok(_) => panic!("workspace should not check"),
            Err(err) => err,
        };
        assert!(
            err.contains("unresolved symbol `hidden` in module `core`"),
            "unexpected error: {err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_workspace_rejects_trait_where_impls_satisfied_only_by_unrelated_members() {
        let root = make_temp_workspace(
            "where_impl_visibility_scope",
            &["app", "core", "stray"],
            &[
                (
                    "app/book.toml",
                    "name = \"app\"\nkind = \"app\"\n\n[deps]\ncore = { path = \"../core\" }\n",
                ),
                (
                    "app/src/shelf.arc",
                    concat!(
                        "use core.Ord\n",
                        "impl Ord[Int] for Int:\n",
                        "    fn cmp(read self: Int) -> Int:\n",
                        "        return 0\n",
                        "fn main() -> Int:\n",
                        "    return 0\n",
                    ),
                ),
                ("app/src/types.arc", ""),
                ("core/book.toml", "name = \"core\"\nkind = \"lib\"\n"),
                (
                    "core/src/book.arc",
                    concat!(
                        "export trait Eq[T]:\n",
                        "    fn ok(read self: T) -> Int:\n",
                        "        return 0\n",
                        "export trait Ord[T, where Eq[T]]:\n",
                        "    fn cmp(read self: T) -> Int:\n",
                        "        return 0\n",
                    ),
                ),
                ("core/src/types.arc", ""),
                (
                    "stray/book.toml",
                    "name = \"stray\"\nkind = \"lib\"\n\n[deps]\ncore = { path = \"../core\" }\n",
                ),
                (
                    "stray/src/book.arc",
                    concat!(
                        "use core.Eq\n",
                        "impl Eq[Int] for Int:\n",
                        "    fn ok(read self: Int) -> Int:\n",
                        "        return 0\n",
                    ),
                ),
                ("stray/src/types.arc", ""),
            ],
        );

        let graph = load_workspace_graph(&root).expect("load graph");
        let err = match check_workspace_graph(&graph) {
            Ok(_) => panic!("workspace should not check"),
            Err(err) => err,
        };
        assert!(
            err.contains("impl requires satisfying where-bound `Eq[Int]`"),
            "unexpected error: {err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn typed_api_fingerprint_rebuilds_dependents_for_boundary_contract_changes() {
        let root = make_temp_workspace(
            "typed_api_boundary_contract",
            &["app", "core"],
            &[
                (
                    "app/book.toml",
                    "name = \"app\"\nkind = \"app\"\n\n[deps]\ncore = { path = \"../core\" }\n",
                ),
                (
                    "app/src/shelf.arc",
                    "import core\nfn main() -> Int:\n    return core.boundary_value :: 1 :: call\n",
                ),
                ("app/src/types.arc", ""),
                ("core/book.toml", "name = \"core\"\nkind = \"lib\"\n"),
                (
                    "core/src/book.arc",
                    "#boundary[target = \"lua\"]\nexport fn boundary_value(value: Int) -> Int:\n    return value\n",
                ),
                ("core/src/types.arc", ""),
            ],
        );

        let graph = load_workspace_graph(&root).expect("load graph");
        let core_id = graph
            .member("core")
            .expect("core member should resolve")
            .package_id
            .clone();
        let app_id = graph
            .member("app")
            .expect("app member should resolve")
            .package_id
            .clone();
        let order = plan_workspace(&graph).expect("plan");
        let first_fingerprints = compute_member_fingerprints(&graph).expect("fingerprints");
        let first_statuses = plan_build(&graph, &order, &first_fingerprints, None).expect("plan");
        execute_planned_build(&graph, &first_fingerprints, &first_statuses);
        let lock_path = write_lockfile(&graph, &order, &first_statuses).expect("lock");
        let existing = read_lockfile(&lock_path).expect("read").expect("lock");

        fs::write(
            root.join("core/src/book.arc"),
            "#boundary[target = \"sql\"]\nexport fn boundary_value(value: Int) -> Int:\n    return value\n",
        )
        .expect("rewrite should succeed");

        let second_fingerprints = compute_member_fingerprints(&graph).expect("fingerprints");
        let second_statuses =
            plan_build(&graph, &order, &second_fingerprints, Some(&existing)).expect("plan");
        assert_eq!(second_statuses[0].member(), core_id);
        assert_eq!(second_statuses[0].disposition(), BuildDisposition::Built);
        assert_eq!(second_statuses[1].member(), app_id);
        assert_eq!(second_statuses[1].disposition(), BuildDisposition::Built);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn typed_api_fingerprint_rebuilds_dependents_for_public_impl_method_changes() {
        let root = make_temp_workspace(
            "typed_api_impl_methods",
            &["app", "core"],
            &[
                (
                    "app/book.toml",
                    "name = \"app\"\nkind = \"app\"\n\n[deps]\ncore = { path = \"../core\" }\n",
                ),
                (
                    "app/src/shelf.arc",
                    "import core\nfn main() -> Int:\n    let counter = core.make_counter :: :: call\n    return counter :: 1 :: add\n",
                ),
                ("app/src/types.arc", ""),
                ("core/book.toml", "name = \"core\"\nkind = \"lib\"\n"),
                (
                    "core/src/book.arc",
                    "import types\nuse types.Counter\nexport fn make_counter() -> Counter:\n    return Counter :: value = 0 :: call\n\nimpl Counter:\n    fn add(self: Counter, value: Int) -> Int:\n        return self.value + value\n",
                ),
                (
                    "core/src/types.arc",
                    "export record Counter:\n    value: Int\n",
                ),
            ],
        );

        let graph = load_workspace_graph(&root).expect("load graph");
        let core_id = graph
            .member("core")
            .expect("core member should resolve")
            .package_id
            .clone();
        let app_id = graph
            .member("app")
            .expect("app member should resolve")
            .package_id
            .clone();
        let order = plan_workspace(&graph).expect("plan");
        let first_fingerprints = compute_member_fingerprints(&graph).expect("fingerprints");
        let first_statuses = plan_build(&graph, &order, &first_fingerprints, None).expect("plan");
        execute_planned_build(&graph, &first_fingerprints, &first_statuses);
        let lock_path = write_lockfile(&graph, &order, &first_statuses).expect("lock");
        let existing = read_lockfile(&lock_path).expect("read").expect("lock");

        fs::write(
            root.join("core/src/book.arc"),
            "import types\nuse types.Counter\nexport fn make_counter() -> Counter:\n    return Counter :: value = 0 :: call\n\nimpl Counter:\n    fn add(self: Counter, value: Int, scale: Int) -> Int:\n        return self.value + value + scale\n",
        )
        .expect("rewrite should succeed");

        let second_fingerprints = compute_member_fingerprints(&graph).expect("fingerprints");
        let second_statuses =
            plan_build(&graph, &order, &second_fingerprints, Some(&existing)).expect("plan");
        assert_eq!(second_statuses[0].member(), core_id);
        assert_eq!(second_statuses[0].disposition(), BuildDisposition::Built);
        assert_eq!(second_statuses[1].member(), app_id);
        assert_eq!(second_statuses[1].disposition(), BuildDisposition::Built);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_handles_owned_desktop_grimoire() {
        let summary = check_path(&owned_app_root().join("arcana-desktop"))
            .expect("owned desktop grimoire should check");
        assert!(summary.package_count >= 2);
        assert!(summary.module_count >= 6);
    }

    #[test]
    fn check_path_handles_owned_graphics_grimoire() {
        let summary = check_path(&owned_app_root().join("arcana-graphics"))
            .expect("owned graphics grimoire should check");
        assert!(summary.package_count >= 2);
        assert!(summary.module_count >= 5);
    }

    #[test]
    fn check_path_handles_owned_text_grimoire() {
        let summary = check_path(&owned_app_root().join("arcana-text"))
            .expect("owned text grimoire should check");
        assert!(summary.package_count >= 2);
        assert!(summary.module_count >= 4);
    }

    #[test]
    fn check_path_handles_owned_audio_grimoire() {
        let summary = check_path(&owned_app_root().join("arcana-audio"))
            .expect("owned audio grimoire should check");
        assert!(summary.package_count >= 2);
        assert!(summary.module_count >= 5);
    }

    #[test]
    fn check_path_accepts_builtin_foreword_package() {
        let root = make_temp_package(
            "builtin_foreword_positive",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "#test\nfn smoke() -> Int:\n    return 0\nfn main() -> Int:\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let checked =
            crate::check_workspace_path(&root).expect("builtin foreword package should check");
        assert_eq!(checked.summary().package_count, 1);
        assert_eq!(checked.summary().module_count, 2);
        assert!(
            checked
                .foreword_catalog()
                .iter()
                .any(|entry| entry.exposed_name == "deprecated"
                    && entry.tier == "builtin"
                    && entry.provider_package_id == BUILTIN_FOREWORD_PROVIDER_PACKAGE_ID),
            "catalog should expose builtin forewords through the shared registry path"
        );
        assert!(
            checked
                .foreword_catalog()
                .iter()
                .any(|entry| entry.exposed_name == "test" && entry.targets == vec!["fn"]),
            "catalog should expose builtin target law"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_workspace_requires_opt_in_for_executable_dependency_forewords() {
        let script_seed_dir = test_temp_dir("arcana-frontend-workspaces", "opt_in_metadata_seed");
        let adapter_rel_path = write_foreword_adapter_script(
            &script_seed_dir,
            "opt_in_metadata_adapter",
            "{\"version\":\"arcana-foreword-stdio-v1\",\"diagnostics\":[]}\n",
        );
        let root = make_temp_workspace(
            "executable_foreword_opt_in",
            &["app", "tool"],
            &[
                (
                    "tool/book.toml",
                    &format!(
                        "name = \"tool\"\nkind = \"lib\"\n[toolchain.foreword_products.tool-forewords]\npath = \"{}\"\n",
                        adapter_rel_path
                    ),
                ),
                (
                    "tool/src/book.arc",
                    concat!(
                        "foreword tool.exec.trace:\n",
                        "    tier = executable\n",
                        "    visibility = public\n",
                        "    targets = [fn]\n",
                        "    retention = compile\n",
                        "    payload = [label: Str]\n",
                        "    handler = tool.exec.trace_handler\n",
                        "foreword handler tool.exec.trace_handler:\n",
                        "    protocol = \"stdio-v1\"\n",
                        "    product = \"tool-forewords\"\n",
                        "    entry = \"trace\"\n",
                    ),
                ),
                ("tool/src/types.arc", ""),
                (
                    "app/book.toml",
                    "name = \"app\"\nkind = \"app\"\n[deps]\ntool = { path = \"../tool\" }\n",
                ),
                (
                    "app/src/shelf.arc",
                    "#tool.exec.trace[label = \"entry\"]\nfn main() -> Int:\n    return 0\n",
                ),
                ("app/src/types.arc", ""),
            ],
        );
        let tool_script_target = root.join("tool").join(&adapter_rel_path);
        if let Some(parent) = tool_script_target.parent() {
            fs::create_dir_all(parent).expect("adapter dir should be creatable");
        }
        copy_test_adapter_script(
            &script_seed_dir.join(&adapter_rel_path),
            &tool_script_target,
        );

        let err = match crate::check_workspace_path(&root) {
            Ok(_) => panic!("executable foreword dependency should require opt-in"),
            Err(err) => err,
        };
        assert!(
            err.contains("requires `executable_forewords = true`")
                || err.contains("requires `executable_forewords = true` on the dependency"),
            "{err}"
        );

        fs::write(
            root.join("app").join("book.toml"),
            "name = \"app\"\nkind = \"app\"\n[deps]\ntool = { path = \"../tool\", executable_forewords = true }\n",
        )
        .expect("manifest should update");
        let checked = crate::check_workspace_path(&root)
            .expect("workspace should accept opted-in executable foreword");
        assert!(
            checked
                .foreword_catalog()
                .iter()
                .any(|entry| entry.exposed_name == "tool.exec.trace"),
            "catalog should include executable foreword"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
        fs::remove_dir_all(script_seed_dir).expect("cleanup should succeed");
    }

    #[test]
    fn check_workspace_accepts_reexported_basic_forewords() {
        let root = make_temp_workspace(
            "basic_foreword_reexport",
            &["app", "bridge", "tool"],
            &[
                ("tool/book.toml", "name = \"tool\"\nkind = \"lib\"\n"),
                (
                    "tool/src/book.arc",
                    concat!(
                        "foreword tool.meta.trace:\n",
                        "    tier = basic\n",
                        "    visibility = public\n",
                        "    targets = [fn]\n",
                        "    retention = runtime\n",
                        "    payload = [label: Str]\n",
                    ),
                ),
                ("tool/src/types.arc", ""),
                (
                    "bridge/book.toml",
                    "name = \"bridge\"\nkind = \"lib\"\n[deps]\ntool = { path = \"../tool\" }\n",
                ),
                (
                    "bridge/src/book.arc",
                    "foreword reexport bridge.meta.trace = tool.meta.trace\n",
                ),
                ("bridge/src/types.arc", ""),
                (
                    "app/book.toml",
                    "name = \"app\"\nkind = \"app\"\n[deps]\nbridge = { path = \"../bridge\" }\n",
                ),
                (
                    "app/src/shelf.arc",
                    "#bridge.meta.trace[label = \"entry\"]\nfn main() -> Int:\n    return 0\n",
                ),
                ("app/src/types.arc", ""),
            ],
        );

        let checked = crate::check_workspace_path(&root)
            .expect("workspace should accept reexported basic foreword");
        assert!(
            checked
                .foreword_catalog()
                .iter()
                .any(|entry| entry.exposed_name == "bridge.meta.trace"),
            "catalog should expose reexported foreword name"
        );
        let index_entry = checked
            .foreword_index()
            .iter()
            .find(|entry| {
                entry.entry_kind == "attached"
                    && entry.qualified_name == "bridge.meta.trace"
                    && entry.target_path == "app.main"
            })
            .expect("reexported foreword should index against the app target");
        assert_eq!(index_entry.retention, "runtime");
        assert_eq!(index_entry.args, vec!["label=\"entry\"".to_string()]);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_workspace_accepts_package_local_foreword_aliases() {
        let root = make_temp_package(
            "aliasapp",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    concat!(
                        "foreword aliasapp.meta.trace:\n",
                        "    tier = basic\n",
                        "    visibility = package\n",
                        "    targets = [fn]\n",
                        "    retention = runtime\n",
                        "    payload = [label: Str]\n",
                        "foreword alias aliasapp.meta.local = aliasapp.meta.trace\n",
                        "#aliasapp.meta.local[label = \"entry\"]\n",
                        "fn main() -> Int:\n",
                        "    return 0\n",
                    ),
                ),
                ("src/types.arc", ""),
            ],
        );

        let checked = crate::check_workspace_path(&root)
            .expect("workspace should accept package-local foreword aliases");
        assert!(
            checked
                .foreword_catalog()
                .iter()
                .any(|entry| entry.exposed_name == "aliasapp.meta.local"),
            "catalog should expose the package-local alias name"
        );
        let index_entry = checked
            .foreword_index()
            .iter()
            .find(|entry| {
                entry.entry_kind == "attached"
                    && entry.qualified_name == "aliasapp.meta.local"
                    && entry.target_path == "aliasapp.main"
            })
            .expect("alias application should appear in the foreword index");
        assert_eq!(index_entry.retention, "runtime");
        assert_eq!(index_entry.args, vec!["label=\"entry\"".to_string()]);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_workspace_indexes_field_and_param_forewords() {
        let root = make_temp_package(
            "field_param_foreword_targets",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    concat!(
                        "foreword field_param_foreword_targets.meta.trace:\n",
                        "    tier = basic\n",
                        "    visibility = public\n",
                        "    targets = [fn, field, param]\n",
                        "    retention = runtime\n",
                        "    payload = [label: Str]\n",
                        "record Box:\n",
                        "    #field_param_foreword_targets.meta.trace[label = \"field\"]\n",
                        "    value: Int\n",
                        "fn helper(#field_param_foreword_targets.meta.trace[label = \"param\"] value: Int) -> Int:\n",
                        "    return value\n",
                        "#field_param_foreword_targets.meta.trace[label = \"fn\"]\n",
                        "fn main() -> Int:\n",
                        "    return helper :: 7 :: call\n",
                    ),
                ),
                ("src/types.arc", ""),
            ],
        );

        let checked = crate::check_workspace_path(&root)
            .expect("workspace should index fn, field, and param foreword targets");
        let index = checked.foreword_index();

        let field_entry = index
            .iter()
            .find(|entry| {
                entry.target_kind == "field"
                    && entry.target_path == "field_param_foreword_targets.Box.value"
                    && entry.qualified_name == "field_param_foreword_targets.meta.trace"
            })
            .expect("field foreword should index");
        assert_eq!(field_entry.retention, "runtime");
        assert_eq!(field_entry.args, vec!["label=\"field\"".to_string()]);

        let param_entry = index
            .iter()
            .find(|entry| {
                entry.target_kind == "param"
                    && entry.target_path == "field_param_foreword_targets.helper(value)"
                    && entry.qualified_name == "field_param_foreword_targets.meta.trace"
            })
            .expect("param foreword should index");
        assert_eq!(param_entry.retention, "runtime");
        assert_eq!(param_entry.args, vec!["label=\"param\"".to_string()]);

        let fn_entry = index
            .iter()
            .find(|entry| {
                entry.target_kind == "fn"
                    && entry.target_path == "field_param_foreword_targets.main"
                    && entry.qualified_name == "field_param_foreword_targets.meta.trace"
            })
            .expect("function foreword should index");
        assert_eq!(fn_entry.retention, "runtime");
        assert_eq!(fn_entry.args, vec!["label=\"fn\"".to_string()]);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_workspace_emits_basic_foreword_registration_rows() {
        let root = make_temp_package(
            "basic_foreword_registration_rows",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    concat!(
                        "foreword basic_foreword_registration_rows.meta.trace:\n",
                        "    tier = basic\n",
                        "    visibility = public\n",
                        "    targets = [fn]\n",
                        "    retention = runtime\n",
                        "    payload = [label: Str]\n",
                        "#basic_foreword_registration_rows.meta.trace[label = \"entry\"]\n",
                        "export fn main() -> Int:\n",
                        "    return 0\n",
                    ),
                ),
                ("src/types.arc", ""),
            ],
        );

        let checked = crate::check_workspace_path(&root)
            .expect("workspace should emit deterministic basic foreword registration rows");
        let registrations = checked.foreword_registrations().to_vec();
        assert_eq!(registrations.len(), 1);
        assert_eq!(
            registrations[0].namespace,
            "basic_foreword_registration_rows.meta.trace"
        );
        assert_eq!(registrations[0].key, "label");
        assert_eq!(registrations[0].value, "\"entry\"");
        assert_eq!(registrations[0].target_kind, "fn");
        assert_eq!(
            registrations[0].target_path,
            "basic_foreword_registration_rows.main"
        );
        assert!(registrations[0].public);
        assert_eq!(
            registrations[0].generated_by.resolved_name,
            "basic_foreword_registration_rows.meta.trace"
        );

        let (workspace, _) = checked.into_workspace_parts();
        let app = workspace
            .package("basic_foreword_registration_rows")
            .expect("package should load");
        let module = app
            .summary
            .modules
            .iter()
            .find(|module| module.module_id == "basic_foreword_registration_rows")
            .expect("app module should exist");
        assert_eq!(module.foreword_registrations.len(), 1);
        assert_eq!(
            module.foreword_registrations[0].namespace,
            "basic_foreword_registration_rows.meta.trace"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_workspace_routes_executable_foreword_warnings_through_namespaced_lint_policy() {
        let script_seed_dir = test_temp_dir("arcana-frontend-workspaces", "lint_policy_seed");
        let adapter_rel_path = write_foreword_adapter_script(
            &script_seed_dir,
            "warning_adapter",
            "{\"version\":\"arcana-foreword-stdio-v1\",\"diagnostics\":[{\"severity\":\"warning\",\"lint\":\"detail\",\"message\":\"adapter note\"}],\"replace_owner\":\"fn main() -> Int:\\n    return 0\\n\",\"append_symbols\":[],\"append_impls\":[]}\n",
        );
        let root = make_temp_workspace(
            "executable_foreword_lint_policy",
            &["app", "tool"],
            &[
                (
                    "tool/book.toml",
                    &format!(
                        "name = \"tool\"\nkind = \"lib\"\n[toolchain.foreword_products.tool-forewords]\npath = \"{}\"\n",
                        adapter_rel_path
                    ),
                ),
                (
                    "tool/src/book.arc",
                    concat!(
                        "foreword tool.exec.trace:\n",
                        "    tier = executable\n",
                        "    visibility = public\n",
                        "    action = transform\n",
                        "    targets = [fn]\n",
                        "    retention = compile\n",
                        "    diagnostic_namespace = tool.exec.trace\n",
                        "    handler = tool.exec.trace_handler\n",
                        "foreword handler tool.exec.trace_handler:\n",
                        "    protocol = \"stdio-v1\"\n",
                        "    product = \"tool-forewords\"\n",
                        "    entry = \"trace\"\n",
                    ),
                ),
                ("tool/src/types.arc", ""),
                (
                    "app/book.toml",
                    "name = \"app\"\nkind = \"app\"\n[deps]\ntool = { path = \"../tool\", executable_forewords = true }\n",
                ),
                (
                    "app/src/shelf.arc",
                    "#tool.exec.trace\nfn main() -> Int:\n    return 0\n",
                ),
                ("app/src/types.arc", ""),
            ],
        );
        let tool_script_target = root.join("tool").join(&adapter_rel_path);
        if let Some(parent) = tool_script_target.parent() {
            fs::create_dir_all(parent).expect("adapter dir should be creatable");
        }
        copy_test_adapter_script(
            &script_seed_dir.join(&adapter_rel_path),
            &tool_script_target,
        );

        let checked = crate::check_workspace_path(&root)
            .expect("workspace should surface adapter warning by default");
        assert!(
            checked
                .warnings()
                .iter()
                .any(|warning| warning.message.contains("adapter note")),
            "default policy should surface the adapter warning"
        );

        fs::write(
            root.join("app").join("src").join("shelf.arc"),
            "#allow[tool.exec.trace]\n#tool.exec.trace\nfn main() -> Int:\n    return 0\n",
        )
        .expect("app module should update");
        let checked = crate::check_workspace_path(&root)
            .expect("allow policy should suppress namespaced adapter warnings");
        assert!(
            !checked
                .warnings()
                .iter()
                .any(|warning| warning.message.contains("adapter note")),
            "allow policy should suppress the adapter warning"
        );

        fs::write(
            root.join("app").join("src").join("shelf.arc"),
            "#deny[tool.exec.trace.detail]\n#tool.exec.trace\nfn main() -> Int:\n    return 0\n",
        )
        .expect("app module should update");
        let err = match crate::check_workspace_path(&root) {
            Ok(_) => panic!("deny policy should escalate namespaced adapter warnings to errors"),
            Err(err) => err,
        };
        assert!(err.contains("adapter note"), "{err}");

        fs::remove_dir_all(root).expect("cleanup should succeed");
        fs::remove_dir_all(script_seed_dir).expect("cleanup should succeed");
    }

    #[test]
    fn check_workspace_routes_basic_foreword_warnings_through_namespaced_lint_policy() {
        let root = make_temp_package(
            "basic_foreword_warning_lint",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    concat!(
                        "foreword basic_foreword_warning_lint.meta.experimental:\n",
                        "    tier = basic\n",
                        "    visibility = public\n",
                        "    targets = [fn]\n",
                        "    retention = compile\n",
                        "    diagnostic_namespace = basic_foreword_warning_lint.meta\n",
                        "#basic_foreword_warning_lint.meta.experimental\n",
                        "fn main() -> Int:\n",
                        "    return 0\n",
                    ),
                ),
                ("src/types.arc", ""),
            ],
        );

        let checked = crate::check_workspace_path(&root)
            .expect("basic foreword warning lane should warn by default");
        assert!(
            checked
                .warnings()
                .iter()
                .any(|warning| warning.message.contains("basic foreword")),
            "default policy should surface the basic foreword warning"
        );

        fs::write(
            root.join("src").join("shelf.arc"),
            concat!(
                "foreword basic_foreword_warning_lint.meta.experimental:\n",
                "    tier = basic\n",
                "    visibility = public\n",
                "    targets = [fn]\n",
                "    retention = compile\n",
                "    diagnostic_namespace = basic_foreword_warning_lint.meta\n",
                "#allow[basic_foreword_warning_lint.meta]\n",
                "#basic_foreword_warning_lint.meta.experimental\n",
                "fn main() -> Int:\n",
                "    return 0\n",
            ),
        )
        .expect("package source should update");
        let checked = crate::check_workspace_path(&root)
            .expect("allow policy should suppress basic foreword warnings");
        assert!(
            !checked
                .warnings()
                .iter()
                .any(|warning| warning.message.contains("basic foreword")),
            "allow policy should suppress the basic foreword warning"
        );

        fs::write(
            root.join("src").join("shelf.arc"),
            concat!(
                "foreword basic_foreword_warning_lint.meta.experimental:\n",
                "    tier = basic\n",
                "    visibility = public\n",
                "    targets = [fn]\n",
                "    retention = compile\n",
                "    diagnostic_namespace = basic_foreword_warning_lint.meta\n",
                "#deny[basic_foreword_warning_lint.meta]\n",
                "#basic_foreword_warning_lint.meta.experimental\n",
                "fn main() -> Int:\n",
                "    return 0\n",
            ),
        )
        .expect("package source should update");
        let err = match crate::check_workspace_path(&root) {
            Ok(_) => panic!("deny policy should escalate the basic foreword warning"),
            Err(err) => err,
        };
        assert!(err.contains("basic foreword"), "{err}");

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_workspace_discovers_tests_and_warns_on_deprecated_use() {
        let root = make_temp_package(
            "deprecated_test_discovery",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    concat!(
                        "#deprecated[\"use fresh\"]\n",
                        "fn old() -> Int:\n",
                        "    return 7\n",
                        "#test\n",
                        "fn smoke() -> Int:\n",
                        "    return old :: :: call\n",
                        "#allow[deprecated_use]\n",
                        "fn quiet() -> Int:\n",
                        "    return old :: :: call\n",
                        "fn main() -> Int:\n",
                        "    return 0\n",
                    ),
                ),
                ("src/types.arc", ""),
            ],
        );

        let checked = crate::check_workspace_path(&root)
            .expect("workspace should discover tests and surface deprecated warnings");
        assert_eq!(checked.discovered_tests().len(), 1);
        assert_eq!(
            checked.discovered_tests()[0].module_id,
            "deprecated_test_discovery"
        );
        assert_eq!(checked.discovered_tests()[0].symbol_name, "smoke");
        assert_eq!(checked.warnings().len(), 1);
        assert!(
            checked.warnings()[0]
                .message
                .contains("use of deprecated `old`: use fresh"),
            "deprecated warning should include the foreword message"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_workspace_denies_deprecated_use_via_builtin_lint_policy() {
        let root = make_temp_package(
            "deprecated_use_deny",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    concat!(
                        "#deprecated[\"use fresh\"]\n",
                        "fn old() -> Int:\n",
                        "    return 7\n",
                        "#deny[deprecated_use]\n",
                        "fn main() -> Int:\n",
                        "    return old :: :: call\n",
                    ),
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = match crate::check_workspace_path(&root) {
            Ok(_) => panic!("deny policy should escalate deprecated-use warnings"),
            Err(err) => err,
        };
        assert!(err.contains("use of deprecated `old`: use fresh"), "{err}");

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn foreword_adapter_cache_keys_change_when_registry_opt_in_or_artifact_identity_changes() {
        let root = test_temp_dir("arcana-frontend-tests", "adapter_cache_key");
        let product_path = root.join("forewords").join("rewrite.sh");
        let runner_path = root.join("forewords").join("runner.sh");
        fs::create_dir_all(product_path.parent().expect("product parent"))
            .expect("product dir should be creatable");
        fs::write(&product_path, "#!/bin/sh\nprintf '{}'\n").expect("product should write");
        fs::write(&runner_path, "#!/bin/sh\nexec \"$@\"\n").expect("runner should write");

        let package = arcana_hir::HirWorkspacePackage {
            package_id: "app.pkg".to_string(),
            root_dir: root.clone(),
            direct_deps: BTreeSet::new(),
            direct_dep_packages: BTreeMap::new(),
            direct_dep_ids: BTreeMap::new(),
            executable_foreword_deps: BTreeSet::new(),
            foreword_products: BTreeMap::new(),
            summary: arcana_hir::HirPackageSummary {
                package_name: "app".to_string(),
                modules: Vec::new(),
                dependency_edges: Vec::new(),
            },
            layout: arcana_hir::HirPackageLayout {
                module_paths: BTreeMap::new(),
                relative_modules: BTreeMap::new(),
                absolute_modules: BTreeMap::new(),
            },
        };
        let definition = arcana_hir::HirForewordDefinition {
            qualified_name: vec![
                "tool".to_string(),
                "exec".to_string(),
                "rewrite".to_string(),
            ],
            tier: arcana_hir::HirForewordTier::Executable,
            visibility: arcana_hir::HirForewordVisibility::Public,
            phase: arcana_hir::HirForewordPhase::Frontend,
            action: arcana_hir::HirForewordAction::Transform,
            targets: vec![arcana_hir::HirForewordDefinitionTarget::Function],
            retention: arcana_hir::HirForewordRetention::Compile,
            payload: vec![arcana_hir::HirForewordPayloadField {
                name: "label".to_string(),
                optional: false,
                ty: arcana_hir::HirForewordPayloadType::Str,
            }],
            repeatable: false,
            conflicts: Vec::new(),
            diagnostic_namespace: Some("tool.exec.trace".to_string()),
            handler: Some(vec![
                "tool".to_string(),
                "exec".to_string(),
                "rewrite_handler".to_string(),
            ]),
            span: Span { line: 1, column: 1 },
        };
        let handler = arcana_hir::HirForewordHandler {
            qualified_name: vec![
                "tool".to_string(),
                "exec".to_string(),
                "rewrite_handler".to_string(),
            ],
            phase: arcana_hir::HirForewordPhase::Frontend,
            protocol: "stdio-v1".to_string(),
            product: "tool-forewords".to_string(),
            entry: "rewrite".to_string(),
            span: Span { line: 1, column: 1 },
        };
        let export = ResolvedForewordExport {
            kind: ResolvedForewordExportKind::User,
            provider_package_id: "tool.pkg".to_string(),
            exposed_package_id: "tool.pkg".to_string(),
            exposed_name: vec![
                "tool".to_string(),
                "exec".to_string(),
                "rewrite".to_string(),
            ],
            definition,
            handler: Some(handler.clone()),
            public: true,
        };
        let target = AdapterTargetSnapshot {
            kind: "fn".to_string(),
            path: "app.main".to_string(),
            public: true,
            owner_kind: "symbol".to_string(),
            owner_symbol: None,
            owner_directive: None,
            selected_field: None,
            selected_param: None,
            selected_method_name: None,
            container_kind: None,
            container_name: None,
        };
        let visible_catalog = vec![
            AdapterCatalogEntry {
                exposed_name: "deprecated".to_string(),
                qualified_name: "deprecated".to_string(),
                tier: "builtin".to_string(),
                action: "metadata".to_string(),
                retention: "compile".to_string(),
                targets: vec!["fn".to_string()],
                provider_package_id: BUILTIN_FOREWORD_PROVIDER_PACKAGE_ID.to_string(),
            },
            AdapterCatalogEntry {
                exposed_name: "tool.exec.rewrite".to_string(),
                qualified_name: "tool.exec.rewrite".to_string(),
                tier: "executable".to_string(),
                action: "transform".to_string(),
                retention: "compile".to_string(),
                targets: vec!["fn".to_string()],
                provider_package_id: "tool.pkg".to_string(),
            },
        ];

        let base_product = arcana_hir::HirForewordAdapterProduct {
            name: "tool-forewords".to_string(),
            path: "forewords/rewrite.sh".to_string(),
            runner: Some("forewords/runner.sh".to_string()),
            args: vec!["--mode".to_string(), "rewrite".to_string()],
        };
        let base_artifact = build_adapter_artifact_identity(&package, &base_product);
        let base_key = build_foreword_adapter_cache_key(
            &package,
            &export,
            &handler,
            &base_product,
            &target,
            &["label=\"entry\"".to_string()],
            &visible_catalog,
            true,
            &base_artifact,
        );

        let mut changed_catalog = visible_catalog.clone();
        changed_catalog.push(AdapterCatalogEntry {
            exposed_name: "tool.meta.trace".to_string(),
            qualified_name: "tool.meta.trace".to_string(),
            tier: "basic".to_string(),
            action: "metadata".to_string(),
            retention: "runtime".to_string(),
            targets: vec!["fn".to_string()],
            provider_package_id: "tool.pkg".to_string(),
        });
        let changed_catalog_key = build_foreword_adapter_cache_key(
            &package,
            &export,
            &handler,
            &base_product,
            &target,
            &["label=\"entry\"".to_string()],
            &changed_catalog,
            true,
            &base_artifact,
        );
        assert_ne!(base_key, changed_catalog_key);

        let changed_opt_in_key = build_foreword_adapter_cache_key(
            &package,
            &export,
            &handler,
            &base_product,
            &target,
            &["label=\"entry\"".to_string()],
            &visible_catalog,
            false,
            &base_artifact,
        );
        assert_ne!(base_key, changed_opt_in_key);

        let changed_product = arcana_hir::HirForewordAdapterProduct {
            args: vec!["--mode".to_string(), "rewrite2".to_string()],
            ..base_product.clone()
        };
        let changed_artifact = build_adapter_artifact_identity(&package, &changed_product);
        let changed_artifact_key = build_foreword_adapter_cache_key(
            &package,
            &export,
            &handler,
            &changed_product,
            &target,
            &["label=\"entry\"".to_string()],
            &visible_catalog,
            true,
            &changed_artifact,
        );
        assert_ne!(base_key, changed_artifact_key);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn lower_adapter_payload_args_preserves_typed_values() {
        let args = vec![
            arcana_hir::HirForewordArg {
                name: Some("enabled".to_string()),
                value: "true".to_string(),
                typed_value: arcana_hir::HirForewordArgValue::Bool(true),
            },
            arcana_hir::HirForewordArg {
                name: Some("count".to_string()),
                value: "7".to_string(),
                typed_value: arcana_hir::HirForewordArgValue::Int(7),
            },
            arcana_hir::HirForewordArg {
                name: Some("label".to_string()),
                value: "\"hello\"".to_string(),
                typed_value: arcana_hir::HirForewordArgValue::Str("hello".to_string()),
            },
            arcana_hir::HirForewordArg {
                name: Some("path".to_string()),
                value: "tool.meta.trace".to_string(),
                typed_value: arcana_hir::HirForewordArgValue::Path(vec![
                    "tool".to_string(),
                    "meta".to_string(),
                    "trace".to_string(),
                ]),
            },
        ];
        let lowered = lower_adapter_payload_args(&args);
        let json = serde_json::to_value(&lowered).expect("payload args should serialize");
        assert_eq!(json[0]["name"], "enabled");
        assert_eq!(json[0]["rendered"], "true");
        assert_eq!(json[0]["value"]["kind"], "Bool");
        assert_eq!(json[0]["value"]["value"], true);
        assert_eq!(json[1]["value"]["kind"], "Int");
        assert_eq!(json[1]["value"]["value"], 7);
        assert_eq!(json[2]["value"]["kind"], "Str");
        assert_eq!(json[2]["value"]["value"], "hello");
        assert_eq!(json[3]["value"]["kind"], "Path");
        assert_eq!(
            json[3]["value"]["value"],
            serde_json::json!(["tool", "meta", "trace"])
        );
    }

    #[test]
    fn build_adapter_artifact_identity_materializes_product_under_package_cache() {
        let root = test_temp_dir("arcana-frontend-tests", "materialized_adapter_identity");
        let adapter_rel_path = write_foreword_adapter_script(
            &root,
            "materialized_identity",
            "{\"version\":\"arcana-foreword-stdio-v1\",\"diagnostics\":[]}\n",
        );
        let (package, product, _) = make_adapter_request_fixture(&root, &adapter_rel_path);
        let identity = build_adapter_artifact_identity(&package, &product);
        assert!(
            identity.product_path.contains(".arcana"),
            "materialized product path should live under package cache: {}",
            identity.product_path
        );
        assert!(
            Path::new(&identity.product_path).is_file(),
            "materialized product should exist at {}",
            identity.product_path
        );
        let materialized = materialize_foreword_adapter_artifact(&package, &product)
            .expect("adapter artifact should materialize");
        assert_eq!(identity.product_path, materialized.product_arg_path);
        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn execute_foreword_adapter_rejects_invalid_json_response() {
        let root = test_temp_dir("arcana-frontend-tests", "adapter_invalid_json");
        let adapter_rel_path = write_foreword_adapter_script(&root, "invalid_json", "not-json");
        let (package, product, request) = make_adapter_request_fixture(&root, &adapter_rel_path);
        let err = execute_foreword_adapter(&package, &product, &request)
            .expect_err("invalid JSON adapter output should fail");
        assert!(err.contains("returned invalid JSON"), "{err}");
        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn execute_foreword_adapter_rejects_protocol_mismatch_response() {
        let root = test_temp_dir("arcana-frontend-tests", "adapter_protocol_mismatch");
        let adapter_rel_path = write_foreword_adapter_script(
            &root,
            "protocol_mismatch",
            "{\"version\":\"wrong-protocol\",\"diagnostics\":[]}",
        );
        let (package, product, request) = make_adapter_request_fixture(&root, &adapter_rel_path);
        let err = execute_foreword_adapter(&package, &product, &request)
            .expect_err("protocol mismatch should fail");
        assert!(err.contains("returned protocol `wrong-protocol`"), "{err}");
        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn execute_foreword_adapter_surfaces_nonzero_exit_detail() {
        let root = test_temp_dir("arcana-frontend-tests", "adapter_nonzero_exit");
        let script = if cfg!(windows) {
            "@echo off\r\necho failure detail 1>&2\r\nexit /b 9\r\n".to_string()
        } else {
            "#!/bin/sh\necho failure detail >&2\nexit 9\n".to_string()
        };
        let adapter_rel_path = write_foreword_script_contents(&root, "nonzero_exit", &script);
        let (package, product, request) = make_adapter_request_fixture(&root, &adapter_rel_path);
        let err = execute_foreword_adapter(&package, &product, &request)
            .expect_err("nonzero adapter exit should fail");
        assert!(err.contains("failure detail"), "{err}");
        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn execute_executable_foreword_app_reuses_cached_response_for_identical_requests() {
        let root = test_temp_dir("arcana-frontend-tests", "adapter_cached_replay");
        let counter_path = root.join("counter.txt");
        let output = "{\"version\":\"arcana-foreword-stdio-v1\",\"diagnostics\":[],\"emitted_metadata\":[],\"registration_rows\":[]}\n";
        let script = if cfg!(windows) {
            format!(
                "@echo off\r\necho run>>\"{}\"\r\necho {}\r\n",
                counter_path.display(),
                output.trim()
            )
        } else {
            format!(
                "#!/bin/sh\necho run >> \"{}\"\nprintf '%s' '{}'\n",
                counter_path.display(),
                output
            )
        };
        let adapter_rel_path = write_foreword_script_contents(&root, "cached_replay", &script);
        let provider_package = arcana_hir::HirWorkspacePackage {
            package_id: "tool.pkg".to_string(),
            root_dir: root.clone(),
            direct_deps: BTreeSet::new(),
            direct_dep_packages: BTreeMap::new(),
            direct_dep_ids: BTreeMap::new(),
            executable_foreword_deps: BTreeSet::new(),
            foreword_products: BTreeMap::from([(
                "tool-forewords".to_string(),
                arcana_hir::HirForewordAdapterProduct {
                    name: "tool-forewords".to_string(),
                    path: adapter_rel_path.clone(),
                    runner: None,
                    args: Vec::new(),
                },
            )]),
            summary: arcana_hir::HirPackageSummary {
                package_name: "tool".to_string(),
                modules: Vec::new(),
                dependency_edges: Vec::new(),
            },
            layout: arcana_hir::HirPackageLayout {
                module_paths: BTreeMap::new(),
                relative_modules: BTreeMap::new(),
                absolute_modules: BTreeMap::new(),
            },
        };
        let consumer_package = arcana_hir::HirWorkspacePackage {
            package_id: "app.pkg".to_string(),
            root_dir: root.clone(),
            direct_deps: BTreeSet::from(["tool.pkg".to_string()]),
            direct_dep_packages: BTreeMap::from([("tool".to_string(), "tool".to_string())]),
            direct_dep_ids: BTreeMap::from([("tool".to_string(), "tool.pkg".to_string())]),
            executable_foreword_deps: BTreeSet::from(["tool".to_string()]),
            foreword_products: BTreeMap::new(),
            summary: arcana_hir::HirPackageSummary {
                package_name: "app".to_string(),
                modules: Vec::new(),
                dependency_edges: Vec::new(),
            },
            layout: arcana_hir::HirPackageLayout {
                module_paths: BTreeMap::new(),
                relative_modules: BTreeMap::new(),
                absolute_modules: BTreeMap::new(),
            },
        };
        let mut workspace = arcana_hir::HirWorkspaceSummary::default();
        workspace
            .packages
            .insert("app.pkg".to_string(), consumer_package.clone());
        workspace
            .packages
            .insert("tool.pkg".to_string(), provider_package.clone());
        let app = arcana_hir::HirForewordApp {
            name: "note".to_string(),
            path: vec!["tool".to_string(), "exec".to_string(), "note".to_string()],
            args: vec![arcana_hir::HirForewordArg {
                name: Some("label".to_string()),
                value: "\"main\"".to_string(),
                typed_value: arcana_hir::HirForewordArgValue::Str("main".to_string()),
            }],
            span: Span { line: 1, column: 1 },
        };
        let definition = arcana_hir::HirForewordDefinition {
            qualified_name: vec!["tool".to_string(), "exec".to_string(), "note".to_string()],
            tier: arcana_hir::HirForewordTier::Executable,
            visibility: arcana_hir::HirForewordVisibility::Public,
            phase: arcana_hir::HirForewordPhase::Frontend,
            action: arcana_hir::HirForewordAction::Metadata,
            targets: vec![arcana_hir::HirForewordDefinitionTarget::Function],
            retention: arcana_hir::HirForewordRetention::Runtime,
            payload: vec![arcana_hir::HirForewordPayloadField {
                name: "label".to_string(),
                optional: false,
                ty: arcana_hir::HirForewordPayloadType::Str,
            }],
            repeatable: false,
            conflicts: Vec::new(),
            diagnostic_namespace: None,
            handler: Some(vec![
                "tool".to_string(),
                "exec".to_string(),
                "note_handler".to_string(),
            ]),
            span: Span { line: 1, column: 1 },
        };
        let handler = arcana_hir::HirForewordHandler {
            qualified_name: vec![
                "tool".to_string(),
                "exec".to_string(),
                "note_handler".to_string(),
            ],
            phase: arcana_hir::HirForewordPhase::Frontend,
            protocol: "stdio-v1".to_string(),
            product: "tool-forewords".to_string(),
            entry: "note".to_string(),
            span: Span { line: 1, column: 1 },
        };
        let export = ResolvedForewordExport {
            kind: ResolvedForewordExportKind::User,
            provider_package_id: "tool.pkg".to_string(),
            exposed_package_id: "tool.pkg".to_string(),
            exposed_name: vec!["tool".to_string(), "exec".to_string(), "note".to_string()],
            definition,
            handler: Some(handler),
            public: true,
        };
        let mut registry = super::ForewordRegistry::default();
        registry.exports.insert(
            ("tool.pkg".to_string(), "exec.note".to_string()),
            export.clone(),
        );
        registry.catalog.push(super::ForewordCatalogEntry {
            exposed_name: "tool.exec.note".to_string(),
            qualified_name: "tool.exec.note".to_string(),
            tier: "executable".to_string(),
            visibility: "public".to_string(),
            action: "metadata".to_string(),
            retention: "runtime".to_string(),
            targets: vec!["fn".to_string()],
            diagnostic_namespace: None,
            handler: Some("tool.exec.note_handler".to_string()),
            provider_package_id: "tool.pkg".to_string(),
        });

        let target = AdapterTargetSnapshot {
            kind: "fn".to_string(),
            path: "app.main".to_string(),
            public: true,
            owner_kind: "symbol".to_string(),
            owner_symbol: None,
            owner_directive: None,
            selected_field: None,
            selected_param: None,
            selected_method_name: None,
            container_kind: None,
            container_name: None,
        };
        let mut cache = BTreeMap::new();
        let mut warnings = Vec::new();
        let mut errors = Vec::new();
        let policy = super::LintPolicy::default();
        let first = execute_executable_foreword_app(
            &workspace,
            &consumer_package,
            &root.join("app.arc"),
            "app",
            &registry,
            &mut cache,
            &app,
            &export,
            "fn",
            "app.main",
            target.clone(),
            &policy,
            &mut warnings,
            &mut errors,
        )
        .expect("first execution should succeed");
        let second = execute_executable_foreword_app(
            &workspace,
            &consumer_package,
            &root.join("app.arc"),
            "app",
            &registry,
            &mut cache,
            &app,
            &export,
            "fn",
            "app.main",
            target,
            &policy,
            &mut warnings,
            &mut errors,
        )
        .expect("cached execution should succeed");
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(first.response.version, second.response.version);
        let counter = fs::read_to_string(&counter_path).expect("counter file should exist");
        assert_eq!(
            counter.lines().count(),
            1,
            "identical executable foreword input should replay from cache"
        );
        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_workspace_executes_executable_foreword_transform_products() {
        let script_seed_dir = test_temp_dir("arcana-frontend-workspaces", "transform_script_seed");
        let adapter_rel_path = write_foreword_adapter_script(
            &script_seed_dir,
            "transform_adapter",
            "{\"version\":\"arcana-foreword-stdio-v1\",\"diagnostics\":[]}\n",
        );
        let runner_rel_path = write_foreword_runner_script(
            &script_seed_dir,
            "transform_adapter_runner",
            "transform_flag",
            "{\"version\":\"arcana-foreword-stdio-v1\",\"diagnostics\":[],\"replace_owner\":\"fn main() -> Int:\\n    return 7\\n\",\"append_symbols\":[\"fn helper() -> Int:\\n    return 11\\n\"],\"append_impls\":[],\"emitted_metadata\":[{\"qualified_name\":\"tool.exec.runtime_helper\",\"target_kind\":\"fn\",\"target_path\":\"app.helper\",\"retention\":\"runtime\",\"args\":[{\"name\":\"slot\",\"value\":\"\\\"helper\\\"\"}]}],\"registration_rows\":[{\"namespace\":\"tool.exec.registry\",\"key\":\"helper\",\"value\":\"runtime\",\"target_kind\":\"fn\",\"target_path\":\"app.helper\"}]}\n",
        );
        let root = make_temp_workspace(
            "executable_foreword_transform",
            &["app", "tool"],
            &[
                (
                    "tool/book.toml",
                    &format!(
                        concat!(
                            "name = \"tool\"\n",
                            "kind = \"lib\"\n",
                            "[toolchain.foreword_products.tool-forewords]\n",
                            "path = \"{}\"\n",
                            "runner = \"{}\"\n",
                            "args = [{}]\n"
                        ),
                        adapter_rel_path,
                        if cfg!(windows) { "powershell" } else { "sh" },
                        if cfg!(windows) {
                            let runner_arg_path = script_seed_dir
                                .join(&runner_rel_path)
                                .display()
                                .to_string()
                                .replace('\\', "\\\\");
                            format!(
                                "\"-ExecutionPolicy\", \"Bypass\", \"-File\", \"{runner_arg_path}\", \"transform_flag\""
                            )
                        } else {
                            format!(
                                "\"{}\", \"transform_flag\"",
                                script_seed_dir.join(&runner_rel_path).display()
                            )
                        }
                    ),
                ),
                (
                    "tool/src/book.arc",
                    concat!(
                        "foreword tool.exec.rewrite:\n",
                        "    tier = executable\n",
                        "    visibility = public\n",
                        "    action = transform\n",
                        "    targets = [fn]\n",
                        "    retention = compile\n",
                        "    handler = tool.exec.rewrite_handler\n",
                        "foreword handler tool.exec.rewrite_handler:\n",
                        "    protocol = \"stdio-v1\"\n",
                        "    product = \"tool-forewords\"\n",
                        "    entry = \"rewrite\"\n",
                    ),
                ),
                ("tool/src/types.arc", ""),
                (
                    "app/book.toml",
                    "name = \"app\"\nkind = \"app\"\n[deps]\ntool = { path = \"../tool\", executable_forewords = true }\n",
                ),
                (
                    "app/src/shelf.arc",
                    "#tool.exec.rewrite\nfn main() -> Int:\n    return 0\n",
                ),
                ("app/src/types.arc", ""),
            ],
        );
        for rel_path in [&adapter_rel_path, &runner_rel_path] {
            let target = root.join("tool").join(rel_path);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).expect("adapter dir should be creatable");
            }
            copy_test_adapter_script(&script_seed_dir.join(rel_path), &target);
        }

        let checked = crate::check_workspace_path(&root)
            .expect("workspace should run executable foreword transform");
        let index = checked.foreword_index().to_vec();
        let registrations = checked.foreword_registrations().to_vec();
        let (workspace, _) = checked.into_workspace_parts();
        let app = workspace.package("app").expect("app package should load");
        let main_module = app
            .summary
            .modules
            .iter()
            .find(|module| module.symbols.iter().any(|symbol| symbol.name == "main"))
            .expect("main module should exist");
        assert!(
            main_module
                .symbols
                .iter()
                .any(|symbol| symbol.name == "helper"),
            "transform should append helper symbol"
        );
        let helper = main_module
            .symbols
            .iter()
            .find(|symbol| symbol.name == "helper")
            .expect("helper symbol should exist");
        assert!(
            helper.generated_name_key.is_some(),
            "generated siblings should carry a stable generation key"
        );
        let main = main_module
            .symbols
            .iter()
            .find(|symbol| symbol.name == "main")
            .expect("main symbol should exist");
        let generated_entry = index
            .iter()
            .find(|entry| {
                entry.entry_kind == "generated"
                    && entry.qualified_name == "tool.exec.rewrite"
                    && entry.target_path == format!("{}.helper", main_module.module_id)
            })
            .expect("generated helper provenance entry should exist");
        assert_eq!(generated_entry.retention, "compile");
        let generated_by = generated_entry
            .generated_by
            .as_ref()
            .expect("generated provenance should include origin");
        assert_eq!(
            generated_by.owner_path,
            format!("{}.main", main_module.module_id)
        );
        let emitted_entry = index
            .iter()
            .find(|entry| {
                entry.entry_kind == "emitted"
                    && entry.qualified_name == "tool.exec.runtime_helper"
                    && entry.target_path == format!("{}.helper", main_module.module_id)
            })
            .expect("adapter-emitted metadata entry should exist");
        assert_eq!(emitted_entry.retention, "runtime");
        assert_eq!(emitted_entry.args, vec!["slot=\"helper\"".to_string()]);
        assert_eq!(registrations.len(), 1);
        assert_eq!(registrations[0].namespace, "tool.exec.registry");
        assert_eq!(
            registrations[0].target_path,
            format!("{}.helper", main_module.module_id)
        );

        let Some(arcana_hir::HirStatement {
            kind:
                arcana_hir::HirStatementKind::Return {
                    value: Some(arcana_hir::HirExpr::IntLiteral { text, .. }),
                },
            ..
        }) = main.statements.first()
        else {
            panic!("transformed main should return an integer literal");
        };
        assert_eq!(text, "7");

        fs::remove_dir_all(root).expect("cleanup should succeed");
        fs::remove_dir_all(script_seed_dir).expect("cleanup should succeed");
    }

    #[test]
    fn check_workspace_executes_executable_foreword_metadata_products() {
        let script_seed_dir = test_temp_dir("arcana-frontend-workspaces", "metadata_script_seed");
        let adapter_rel_path = write_foreword_adapter_script(
            &script_seed_dir,
            "metadata_adapter",
            "{\"version\":\"arcana-foreword-stdio-v1\",\"diagnostics\":[]}\n",
        );
        let runner_rel_path = write_foreword_runner_script(
            &script_seed_dir,
            "metadata_adapter_runner",
            "metadata_flag",
            "{\"version\":\"arcana-foreword-stdio-v1\",\"diagnostics\":[],\"emitted_metadata\":[{\"qualified_name\":\"tool.exec.runtime_note\",\"target_kind\":\"fn\",\"target_path\":\"app.main\",\"retention\":\"runtime\",\"args\":[{\"name\":\"slot\",\"value\":\"\\\"main\\\"\"}]}],\"registration_rows\":[{\"namespace\":\"tool.exec.registry\",\"key\":\"main\",\"value\":\"metadata\",\"target_kind\":\"fn\",\"target_path\":\"app.main\"}]}\n",
        );
        let root = make_temp_workspace(
            "executable_foreword_metadata",
            &["app", "tool"],
            &[
                (
                    "tool/book.toml",
                    &format!(
                        concat!(
                            "name = \"tool\"\n",
                            "kind = \"lib\"\n",
                            "[toolchain.foreword_products.tool-forewords]\n",
                            "path = \"{}\"\n",
                            "runner = \"{}\"\n",
                            "args = [{}]\n"
                        ),
                        adapter_rel_path,
                        if cfg!(windows) { "powershell" } else { "sh" },
                        if cfg!(windows) {
                            let runner_arg_path = script_seed_dir
                                .join(&runner_rel_path)
                                .display()
                                .to_string()
                                .replace('\\', "\\\\");
                            format!(
                                "\"-ExecutionPolicy\", \"Bypass\", \"-File\", \"{runner_arg_path}\", \"metadata_flag\""
                            )
                        } else {
                            format!(
                                "\"{}\", \"metadata_flag\"",
                                script_seed_dir.join(&runner_rel_path).display()
                            )
                        }
                    ),
                ),
                (
                    "tool/src/book.arc",
                    concat!(
                        "foreword tool.exec.note:\n",
                        "    tier = executable\n",
                        "    visibility = public\n",
                        "    action = metadata\n",
                        "    targets = [fn]\n",
                        "    retention = runtime\n",
                        "    handler = tool.exec.note_handler\n",
                        "foreword handler tool.exec.note_handler:\n",
                        "    protocol = \"stdio-v1\"\n",
                        "    product = \"tool-forewords\"\n",
                        "    entry = \"note\"\n",
                    ),
                ),
                ("tool/src/types.arc", ""),
                (
                    "app/book.toml",
                    "name = \"app\"\nkind = \"app\"\n[deps]\ntool = { path = \"../tool\", executable_forewords = true }\n",
                ),
                (
                    "app/src/shelf.arc",
                    "#tool.exec.note\nfn main() -> Int:\n    return 0\n",
                ),
                ("app/src/types.arc", ""),
            ],
        );
        for rel_path in [&adapter_rel_path, &runner_rel_path] {
            let target = root.join("tool").join(rel_path);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).expect("adapter dir should be creatable");
            }
            copy_test_adapter_script(&script_seed_dir.join(rel_path), &target);
        }

        let checked = crate::check_workspace_path(&root)
            .expect("workspace should run executable foreword metadata adapter");
        let index = checked.foreword_index().to_vec();
        let registrations = checked.foreword_registrations().to_vec();
        let (workspace, _) = checked.into_workspace_parts();
        let app = workspace.package("app").expect("app package should load");
        let module = app
            .summary
            .modules
            .iter()
            .find(|module| module.module_id == "app")
            .expect("app module should exist");
        let main = module
            .symbols
            .iter()
            .find(|symbol| symbol.name == "main")
            .expect("main symbol should exist");
        let Some(arcana_hir::HirStatement {
            kind:
                arcana_hir::HirStatementKind::Return {
                    value: Some(arcana_hir::HirExpr::IntLiteral { text, .. }),
                },
            ..
        }) = main.statements.first()
        else {
            panic!("metadata adapter should not rewrite the owner symbol");
        };
        assert_eq!(text, "0");
        assert!(
            index.iter().any(|entry| {
                entry.entry_kind == "emitted"
                    && entry.qualified_name == "tool.exec.runtime_note"
                    && entry.target_path == "app.main"
            }),
            "metadata adapter should emit retained metadata"
        );
        assert_eq!(registrations.len(), 1);
        assert_eq!(registrations[0].namespace, "tool.exec.registry");
        assert_eq!(registrations[0].value, "metadata");
        assert_eq!(module.emitted_foreword_metadata.len(), 1);

        fs::remove_dir_all(root).expect("cleanup should succeed");
        fs::remove_dir_all(script_seed_dir).expect("cleanup should succeed");
    }

    #[test]
    fn check_workspace_rejects_nonadjacent_directive_sibling_emission() {
        let script_seed_dir = test_temp_dir(
            "arcana-frontend-workspaces",
            "directive_append_rejection_seed",
        );
        let adapter_rel_path = write_foreword_adapter_script(
            &script_seed_dir,
            "directive_append_rejection",
            "{\"version\":\"arcana-foreword-stdio-v1\",\"diagnostics\":[]}\n",
        );
        let runner_rel_path = write_foreword_runner_script(
            &script_seed_dir,
            "directive_append_rejection_runner",
            "reject_flag",
            "{\"version\":\"arcana-foreword-stdio-v1\",\"diagnostics\":[],\"replace_directive\":\"import std.io\",\"append_symbols\":[\"fn helper() -> Int:\\n    return 1\\n\"],\"append_impls\":[]}\n",
        );
        let root = make_temp_workspace(
            "executable_foreword_directive_append_rejection",
            &["app", "tool"],
            &[
                (
                    "tool/book.toml",
                    &format!(
                        concat!(
                            "name = \"tool\"\n",
                            "kind = \"lib\"\n",
                            "[toolchain.foreword_products.tool-forewords]\n",
                            "path = \"{}\"\n",
                            "runner = \"{}\"\n",
                            "args = [{}]\n"
                        ),
                        adapter_rel_path,
                        if cfg!(windows) { "powershell" } else { "sh" },
                        if cfg!(windows) {
                            let runner_arg_path = script_seed_dir
                                .join(&runner_rel_path)
                                .display()
                                .to_string()
                                .replace('\\', "\\\\");
                            format!(
                                "\"-ExecutionPolicy\", \"Bypass\", \"-File\", \"{runner_arg_path}\", \"reject_flag\""
                            )
                        } else {
                            format!(
                                "\"{}\", \"reject_flag\"",
                                script_seed_dir.join(&runner_rel_path).display()
                            )
                        }
                    ),
                ),
                (
                    "tool/src/book.arc",
                    concat!(
                        "foreword tool.exec.expand:\n",
                        "    tier = executable\n",
                        "    visibility = public\n",
                        "    action = transform\n",
                        "    targets = [import]\n",
                        "    retention = compile\n",
                        "    handler = tool.exec.expand_handler\n",
                        "foreword handler tool.exec.expand_handler:\n",
                        "    protocol = \"stdio-v1\"\n",
                        "    product = \"tool-forewords\"\n",
                        "    entry = \"expand\"\n",
                    ),
                ),
                ("tool/src/types.arc", ""),
                (
                    "app/book.toml",
                    "name = \"app\"\nkind = \"app\"\n[deps]\ntool = { path = \"../tool\", executable_forewords = true }\n",
                ),
                (
                    "app/src/shelf.arc",
                    "#tool.exec.expand\nimport std.io\nfn main() -> Int:\n    return 0\n",
                ),
                ("app/src/types.arc", ""),
            ],
        );
        for rel_path in [&adapter_rel_path, &runner_rel_path] {
            let target = root.join("tool").join(rel_path);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).expect("adapter dir should be creatable");
            }
            copy_test_adapter_script(&script_seed_dir.join(rel_path), &target);
        }

        let err = match crate::check_workspace_path(&root) {
            Ok(_) => panic!("directive sibling emission should be rejected"),
            Err(err) => err,
        };
        assert!(err.contains("cannot append sibling declarations"), "{err}");

        fs::remove_dir_all(root).expect("cleanup should succeed");
        fs::remove_dir_all(script_seed_dir).expect("cleanup should succeed");
    }

    #[test]
    fn check_workspace_executes_executable_foreword_transform_products_via_runner_indirection() {
        let script_seed_dir = test_temp_dir("arcana-frontend-workspaces", "transform_runner_seed");
        let adapter_rel_path = write_foreword_adapter_script(
            &script_seed_dir,
            "transform_with_runner",
            "{\"version\":\"arcana-foreword-stdio-v1\",\"diagnostics\":[],\"replace_owner\":\"fn main() -> Int:\\n    return 13\\n\",\"append_symbols\":[],\"append_impls\":[]}\n",
        );
        let runner_rel_path = write_foreword_runner_script(
            &script_seed_dir,
            "transform_runner",
            "runner_flag",
            "{\"version\":\"arcana-foreword-stdio-v1\",\"diagnostics\":[],\"replace_owner\":\"fn main() -> Int:\\n    return 13\\n\",\"append_symbols\":[],\"append_impls\":[]}\n",
        );
        let root = make_temp_workspace(
            "executable_foreword_transform_runner",
            &["app", "tool"],
            &[
                (
                    "tool/book.toml",
                    &format!(
                        concat!(
                            "name = \"tool\"\n",
                            "kind = \"lib\"\n",
                            "[toolchain.foreword_products.tool-forewords]\n",
                            "path = \"{}\"\n",
                            "runner = \"{}\"\n",
                            "args = []\n"
                        ),
                        adapter_rel_path,
                        if cfg!(windows) { "cmd" } else { "sh" }
                    ),
                ),
                (
                    "tool/src/book.arc",
                    concat!(
                        "foreword tool.exec.rewrite:\n",
                        "    tier = executable\n",
                        "    visibility = public\n",
                        "    action = transform\n",
                        "    targets = [fn]\n",
                        "    retention = compile\n",
                        "    handler = tool.exec.rewrite_handler\n",
                        "foreword handler tool.exec.rewrite_handler:\n",
                        "    protocol = \"stdio-v1\"\n",
                        "    product = \"tool-forewords\"\n",
                        "    entry = \"rewrite\"\n",
                    ),
                ),
                ("tool/src/types.arc", ""),
                (
                    "app/book.toml",
                    "name = \"app\"\nkind = \"app\"\n[deps]\ntool = { path = \"../tool\", executable_forewords = true }\n",
                ),
                (
                    "app/src/shelf.arc",
                    "#tool.exec.rewrite\nfn main() -> Int:\n    return 0\n",
                ),
                ("app/src/types.arc", ""),
            ],
        );
        for rel_path in [&adapter_rel_path, &runner_rel_path] {
            let target = root.join("tool").join(rel_path);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).expect("script dir should be creatable");
            }
            copy_test_adapter_script(&script_seed_dir.join(rel_path), &target);
        }
        let runner_arg_path = root
            .join("tool")
            .join(&runner_rel_path)
            .display()
            .to_string()
            .replace('\\', "\\\\");
        fs::write(
            root.join("tool").join("book.toml"),
            format!(
                concat!(
                    "name = \"tool\"\n",
                    "kind = \"lib\"\n",
                    "[toolchain.foreword_products.tool-forewords]\n",
                    "path = \"{}\"\n",
                    "runner = \"{}\"\n",
                    "args = [{}]\n"
                ),
                adapter_rel_path,
                if cfg!(windows) {
                    "powershell"
                } else {
                    "sh"
                },
                if cfg!(windows) {
                    format!(
                        "\"-ExecutionPolicy\", \"Bypass\", \"-File\", \"{runner_arg_path}\", \"runner_flag\""
                    )
                } else {
                    format!("\"{runner_arg_path}\", \"runner_flag\"")
                }
            ),
        )
        .expect("runner manifest should update");

        let checked = crate::check_workspace_path(&root)
            .expect("workspace should run executable foreword via runner indirection");
        let (workspace, _) = checked.into_workspace_parts();
        let app = workspace.package("app").expect("app package should load");
        let main_module = app
            .summary
            .modules
            .iter()
            .find(|module| module.symbols.iter().any(|symbol| symbol.name == "main"))
            .expect("main module should exist");
        let main = main_module
            .symbols
            .iter()
            .find(|symbol| symbol.name == "main")
            .expect("main symbol should exist");

        let Some(arcana_hir::HirStatement {
            kind:
                arcana_hir::HirStatementKind::Return {
                    value: Some(arcana_hir::HirExpr::IntLiteral { text, .. }),
                },
            ..
        }) = main.statements.first()
        else {
            panic!("transformed main should return an integer literal");
        };
        assert_eq!(text, "13");

        fs::remove_dir_all(root).expect("cleanup should succeed");
        fs::remove_dir_all(script_seed_dir).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_non_repeatable_user_forewords_on_same_target() {
        let root = make_temp_package(
            "app",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    concat!(
                        "foreword app.meta.once:\n",
                        "    tier = basic\n",
                        "    visibility = public\n",
                        "    targets = [fn]\n",
                        "    retention = compile\n",
                        "#app.meta.once\n",
                        "#app.meta.once\n",
                        "fn main() -> Int:\n",
                        "    return 0\n",
                    ),
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("duplicate non-repeatable foreword should fail");
        assert!(err.contains("is not repeatable"), "{err}");

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_conflicting_user_forewords_on_same_target() {
        let root = make_temp_package(
            "app",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    concat!(
                        "foreword app.meta.a:\n",
                        "    tier = basic\n",
                        "    visibility = public\n",
                        "    targets = [fn]\n",
                        "    retention = compile\n",
                        "    conflicts = [app.meta.b]\n",
                        "foreword app.meta.b:\n",
                        "    tier = basic\n",
                        "    visibility = public\n",
                        "    targets = [fn]\n",
                        "    retention = compile\n",
                        "#app.meta.a\n",
                        "#app.meta.b\n",
                        "fn main() -> Int:\n",
                        "    return 0\n",
                    ),
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("conflicting forewords should fail");
        assert!(err.contains("conflicts with"), "{err}");

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_accepts_boundary_interop_package() {
        let root = make_temp_package(
            "boundary_interop_positive",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "import types\n#boundary[target = \"lua\"]\nexport fn bridge(read payload: types.Payload) -> Int:\n    return payload.value\nfn main() -> Int:\n    return 0\n",
                ),
                ("src/types.arc", "export record Payload:\n    value: Int\n"),
            ],
        );

        let summary = check_path(&root).expect("boundary interop package should check");
        assert_eq!(summary.package_count, 1);
        assert_eq!(summary.module_count, 2);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_handles_std_intrinsics() {
        let summary = check_path(&repo_root().join("std")).expect("std should check");
        assert!(summary.package_count >= 1);
        assert!(summary.module_count >= 10);
    }

    #[test]
    fn check_path_accepts_cleanup_footer_package() {
        let root = make_temp_package(
            "cleanup_footer_positive",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    concat!(
                        "enum Result[T, E]:\n",
                        "    Ok(T)\n",
                        "    Err(E)\n",
                        "record Box:\n",
                        "    value: Int\n",
                        "trait Cleanup[T]:\n",
                        "    fn cleanup(take self: T) -> Result[Unit, Str]\n",
                        "lang cleanup_contract = Cleanup\n",
                        "impl Cleanup[Box] for Box:\n",
                        "    fn cleanup(take self: Box) -> Result[Unit, Str]:\n",
                        "        return Result.Ok[Unit, Str] :: :: call\n",
                        "fn cleanup(take value: Box) -> Result[Unit, Str]:\n",
                        "    return Result.Ok[Unit, Str] :: :: call\n",
                        "fn run(take seed: Box) -> Int:\n",
                        "    let mut local = seed.value\n",
                        "    while local > 0:\n",
                        "        let scratch = Box :: value = local :: call\n",
                        "        local -= 1\n",
                        "    -cleanup[target = scratch, handler = cleanup]\n",
                        "    return local\n",
                        "-cleanup[target = seed, handler = cleanup]\n",
                        "fn main() -> Int:\n",
                        "    let seed = Box :: value = 1 :: call\n",
                        "    return run :: seed :: call\n",
                    ),
                ),
                ("src/types.arc", ""),
            ],
        );

        let summary = check_path(&root).expect("cleanup footer package should check");
        assert_eq!(summary.package_count, 1);
        assert_eq!(summary.module_count, 2);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_accepts_cleanup_footer_targeting_nested_scope_binding() {
        let root = make_temp_package(
            "cleanup_footer_nested_target",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "enum Result[T, E]:\n    Ok(T)\n    Err(E)\nrecord Box:\n    value: Int\ntrait Cleanup[T]:\n    fn cleanup(take self: T) -> Result[Unit, Str]\nlang cleanup_contract = Cleanup\nimpl Cleanup[Box] for Box:\n    fn cleanup(take self: Box) -> Result[Unit, Str]:\n        return Result.Ok[Unit, Str] :: :: call\nfn cleanup(take value: Box) -> Result[Unit, Str]:\n    return Result.Ok[Unit, Str] :: :: call\nfn main() -> Int:\n    if true:\n        let inner = Box :: value = 2 :: call\n    return 0\n-cleanup[target = inner, handler = cleanup]\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let summary = check_path(&root).expect("nested cleanup footer target should check");
        assert_eq!(summary.package_count, 1);
        assert_eq!(summary.module_count, 2);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_accepts_cleanup_footer_on_system_symbol() {
        let root = make_temp_package(
            "cleanup_footer_system",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "enum Result[T, E]:\n    Ok(T)\n    Err(E)\nrecord Box:\n    value: Int\ntrait Cleanup[T]:\n    fn cleanup(take self: T) -> Result[Unit, Str]\nlang cleanup_contract = Cleanup\nimpl Cleanup[Box] for Box:\n    fn cleanup(take self: Box) -> Result[Unit, Str]:\n        return Result.Ok[Unit, Str] :: :: call\nfn cleanup(take value: Box) -> Result[Unit, Str]:\n    return Result.Ok[Unit, Str] :: :: call\nsystem[phase=startup] fn boot() -> Int:\n    let value = Box :: value = 1 :: call\n    return 0\n-cleanup[target = value, handler = cleanup]\nfn main() -> Int:\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let summary = check_path(&root).expect("system cleanup footer should check");
        assert_eq!(summary.package_count, 1);
        assert_eq!(summary.module_count, 2);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_ambiguous_cleanup_footer_target_under_shadowing() {
        let root = make_temp_package(
            "cleanup_footer_shadowing",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "enum Result[T, E]:\n    Ok(T)\n    Err(E)\nrecord Box:\n    value: Int\ntrait Cleanup[T]:\n    fn cleanup(take self: T) -> Result[Unit, Str]\nlang cleanup_contract = Cleanup\nimpl Cleanup[Box] for Box:\n    fn cleanup(take self: Box) -> Result[Unit, Str]:\n        return Result.Ok[Unit, Str] :: :: call\nfn cleanup(take value: Box) -> Result[Unit, Str]:\n    return Result.Ok[Unit, Str] :: :: call\nfn main() -> Int:\n    let x = Box :: value = 1 :: call\n    if true:\n        let x = Box :: value = 2 :: call\n    return 0\n-cleanup[target = x, handler = cleanup]\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("shadowed cleanup footer target should fail");
        assert!(
            err.contains("cleanup footer target `x` is ambiguous in the owning header scope"),
            "unexpected error: {err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_unknown_cleanup_footer_target() {
        let root = make_temp_package(
            "cleanup_footer_unknown_target",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "enum Result[T, E]:\n    Ok(T)\n    Err(E)\nfn cleanup(take value: Int) -> Result[Unit, Str]:\n    return Result.Ok[Unit, Str] :: :: call\nfn main() -> Int:\n    let value = 1\n    return value\n-cleanup[target = missing, handler = cleanup]\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("unknown cleanup footer target should fail");
        assert!(
            err.contains(
                "cleanup footer target `missing` is not available in the owning header scope"
            ),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_reassigned_cleanup_footer_target() {
        let root = make_temp_package(
            "cleanup_footer_reassigned_target",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    concat!(
                        "enum Result[T, E]:\n",
                        "    Ok(T)\n",
                        "    Err(E)\n",
                        "record Box:\n",
                        "    value: Int\n",
                        "trait Cleanup[T]:\n",
                        "    fn cleanup(take self: T) -> Result[Unit, Str]\n",
                        "lang cleanup_contract = Cleanup\n",
                        "impl Cleanup[Box] for Box:\n",
                        "    fn cleanup(take self: Box) -> Result[Unit, Str]:\n",
                        "        return Result.Ok[Unit, Str] :: :: call\n",
                        "fn cleanup(take value: Box) -> Result[Unit, Str]:\n",
                        "    return Result.Ok[Unit, Str] :: :: call\n",
                        "fn main() -> Int:\n",
                        "    let mut local = Box :: value = 1 :: call\n",
                        "    local = Box :: value = 2 :: call\n",
                        "    return local.value\n",
                        "-cleanup[target = local, handler = cleanup]\n",
                    ),
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("reassigned cleanup footer target should fail");
        assert!(
            err.contains("cleanup footer target `local` cannot be reassigned after activation"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_accepts_cleanup_footer_targeting_owner_activated_object_binding() {
        let root = make_temp_package(
            "cleanup_footer_owner_object_positive",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "enum Result[T, E]:\n    Ok(T)\n    Err(E)\ntrait Cleanup[T]:\n    fn cleanup(take self: T) -> Result[Unit, Str]\nlang cleanup_contract = Cleanup\nobj Counter:\n    value: Int\n    fn init(edit self: Self):\n        self.value = 1\nimpl Cleanup[Counter] for Counter:\n    fn cleanup(take self: Counter) -> Result[Unit, Str]:\n        return Result.Ok[Unit, Str] :: :: call\ncreate Session [Counter] scope-exit:\n    done: when false hold [Counter]\nfn dispose(take value: Counter) -> Result[Unit, Str]:\n    return Result.Ok[Unit, Str] :: :: call\nSession\nCounter\nfn main() -> Int:\n    let active = Session :: :: call\n    return 0\n-cleanup[target = Counter, handler = dispose]\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let summary =
            check_path(&root).expect("owner-activated cleanup footer target should check");
        assert_eq!(summary.package_count, 1);
        assert_eq!(summary.module_count, 2);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_async_cleanup_footer_handler() {
        let root = make_temp_package(
            "cleanup_footer_async_handler",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "enum Result[T, E]:\n    Ok(T)\n    Err(E)\nasync fn cleanup(take value: Int) -> Result[Unit, Str]:\n    return Result.Ok[Unit, Str] :: :: call\nfn main() -> Int:\n    let value = 1\n    return 0\n-cleanup[target = value, handler = cleanup]\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("async cleanup footer handler should fail");
        assert!(
            err.contains("cleanup footer handler `cleanup` cannot be async in v1"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_non_callable_cleanup_footer_handler() {
        let root = make_temp_package(
            "cleanup_footer_non_callable_handler",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "record Cleaner:\n    id: Int\nfn main() -> Int:\n    let value = 1\n    return 0\n-cleanup[target = value, handler = Cleaner]\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("non-callable cleanup footer handler should fail");
        assert!(
            err.contains("cleanup footer handler `Cleaner` must resolve to a callable symbol"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_wrong_arity_cleanup_footer_handler() {
        let root = make_temp_package(
            "cleanup_footer_wrong_arity_handler",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "enum Result[T, E]:\n    Ok(T)\n    Err(E)\nfn cleanup() -> Result[Unit, Str]:\n    return Result.Ok[Unit, Str] :: :: call\nfn main() -> Int:\n    let value = 1\n    return 0\n-cleanup[target = value, handler = cleanup]\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("wrong-arity cleanup footer handler should fail");
        assert!(
            err.contains(
                "cleanup footer handler `cleanup` must accept exactly one parameter in v1"
            ),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_cleanup_subject_move_after_activation() {
        let root = make_temp_package(
            "cleanup_footer_moved_subject",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    concat!(
                        "enum Result[T, E]:\n",
                        "    Ok(T)\n",
                        "    Err(E)\n",
                        "record Box:\n",
                        "    value: Int\n",
                        "trait Cleanup[T]:\n",
                        "    fn cleanup(take self: T) -> Result[Unit, Str]\n",
                        "lang cleanup_contract = Cleanup\n",
                        "impl Cleanup[Box] for Box:\n",
                        "    fn cleanup(take self: Box) -> Result[Unit, Str]:\n",
                        "        return Result.Ok[Unit, Str] :: :: call\n",
                        "fn cleanup(take value: Box) -> Result[Unit, Str]:\n",
                        "    return Result.Ok[Unit, Str] :: :: call\n",
                        "fn consume(take value: Box):\n",
                        "    return\n",
                        "fn main() -> Int:\n",
                        "    let text = Box :: value = 1 :: call\n",
                        "    consume :: text :: call\n",
                        "    return 0\n",
                        "-cleanup[target = text, handler = cleanup]\n",
                    ),
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("moved cleanup subject should fail");
        assert!(
            err.contains("cleanup subject `text` cannot be moved after activation"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_non_cleanup_capable_target_without_activation_follow_on_errors() {
        let root = make_temp_package(
            "cleanup_footer_non_cleanup_capable_target",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "enum Result[T, E]:\n    Ok(T)\n    Err(E)\nfn cleanup(take value: Int) -> Result[Unit, Str]:\n    return Result.Ok[Unit, Str] :: :: call\nfn main(seed: Int) -> Int:\n    let local = seed\n    local += 1\n    return local\n-cleanup[target = local, handler = cleanup]\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err =
            check_path(&root).expect_err("non-cleanup-capable cleanup footer target should fail");
        assert!(
            err.contains(
                "cleanup footer target `local` is not cleanup-capable in the owning header scope"
            ),
            "{err}"
        );
        assert!(
            !err.contains("cleanup footer target `local` cannot be reassigned after activation"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_accepts_std_handle_cleanup_footer_conformance_fixture() {
        let summary = check_path(
            &repo_root()
                .join("conformance")
                .join("fixtures")
                .join("cleanup_footer_std_handle_workspace")
                .join("app"),
        )
        .expect("std-handle cleanup footer fixture should check");
        assert!(summary.package_count >= 2);
        assert!(summary.module_count >= 2);
    }

    #[test]
    fn check_path_accepts_cleanup_footer_examples_package() {
        let summary = check_path(
            &repo_root()
                .join("examples")
                .join("cleanup-footer-examples")
                .join("app"),
        )
        .expect("cleanup footer examples package should check");
        assert!(summary.package_count >= 1);
        assert!(summary.module_count >= 2);
    }

    #[test]
    fn check_path_accepts_desktop_proof_app_with_defer_dispatcher() {
        let summary = check_path(
            &repo_root()
                .join("examples")
                .join("arcana-desktop-proof")
                .join("app"),
        )
        .expect("desktop proof app should check with defer-based dispatcher cleanup");
        assert!(summary.package_count >= 3);
        assert!(summary.module_count >= 2);
    }

    #[test]
    fn check_path_rejects_unresolved_lang_item_package() {
        let err = check_path(
            &repo_root()
                .join("conformance")
                .join("check_parity_packages")
                .join("unresolved_lang_item"),
        )
        .expect_err("unresolved lang item package should fail");
        assert!(
            err.contains("unresolved `lang` item target `Missing` for `result`"),
            "{err}"
        );
    }

    #[test]
    fn check_path_rejects_invalid_boundary_and_test_packages() {
        let repo_root = repo_root()
            .join("conformance")
            .join("check_parity_packages");
        for (package, expected) in [
            (
                "invalid_boundary_signature",
                "`#boundary` target `lua` does not allow mutable borrows",
            ),
            (
                "nested_boundary_unsafe",
                "type `types.Payload` is not boundary-safe for target `lua`",
            ),
            (
                "invalid_test_signature",
                "`#test` functions must have zero parameters",
            ),
        ] {
            let err = check_path(&repo_root.join(package)).expect_err("package should fail");
            assert!(err.contains(expected), "{package}: {err}");
        }
    }

    #[test]
    fn check_path_rejects_unresolved_type_package() {
        let err = check_path(
            &repo_root()
                .join("conformance")
                .join("check_parity_packages")
                .join("unresolved_type_ref"),
        )
        .expect_err("unresolved type package should fail");
        assert!(
            err.contains("unresolved type reference `MissingType` in field type `value`"),
            "{err}"
        );
    }

    #[test]
    fn check_path_rejects_undeclared_lifetime_package() {
        let err = check_path(
            &repo_root()
                .join("conformance")
                .join("check_parity_packages")
                .join("undeclared_lifetime"),
        )
        .expect_err("undeclared lifetime package should fail");
        assert!(
            err.contains("undeclared lifetime `'a` in parameter type `value`"),
            "{err}"
        );
    }

    #[test]
    fn check_path_rejects_unresolved_body_value_packages() {
        let repo_root = repo_root()
            .join("conformance")
            .join("check_parity_packages");
        for (package, expected) in [
            (
                "unresolved_value_ref",
                "unresolved value reference `missing` in value expression",
            ),
            (
                "unresolved_namespace_member_ref",
                "unresolved value reference `std.kernel.text.missing` in value expression",
            ),
            (
                "unresolved_expr_type_arg",
                "unresolved type reference `Missing` in expression generic argument `Missing`",
            ),
            (
                "unresolved_chain_step",
                "unresolved value reference `missing` in chain step `missing`",
            ),
            (
                "unresolved_cleanup_footer_handler",
                "unresolved cleanup footer handler `missing.cleanup`",
            ),
        ] {
            let err = check_path(&repo_root.join(package))
                .expect_err("unresolved body semantic package should fail");
            assert!(err.contains(expected), "{package}: {err}");
        }
    }

    #[test]
    fn check_path_rejects_non_bool_if_condition() {
        let root = make_temp_package(
            "typed_if_condition",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "fn main() -> Int:\n    if 1:\n        return 0\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("non-bool if condition should fail");
        assert!(
            err.contains("if condition requires Bool, found Int"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_non_bool_not_operand() {
        let root = make_temp_package(
            "typed_not_operand",
            "app",
            &[],
            &[
                ("src/shelf.arc", "fn main() -> Bool:\n    return not 1\n"),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("non-bool `not` operand should fail");
        assert!(
            err.contains("operand of `not` requires Bool, found Int"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_non_int_shift_operand() {
        let root = make_temp_package(
            "typed_shift_operand",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "fn main() -> Int:\n    return 1 << \"x\"\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("non-int shift operand should fail");
        assert!(
            err.contains("right operand of `<<` requires Int, found Str"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_tuple_projection_on_non_pair() {
        let root = make_temp_package(
            "typed_tuple_projection",
            "app",
            &[],
            &[
                ("src/shelf.arc", "fn main() -> Int:\n    return (\"x\").0\n"),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("tuple projection on non-pair should fail");
        assert!(
            err.contains("tuple field access `.0` requires a pair value, found Str"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_borrow_of_non_place_expression() {
        let root = make_temp_package(
            "typed_non_place_borrow",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "fn main() -> Int:\n    let x = &(1 + 2)\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("borrow of non-place should fail");
        assert!(
            err.contains("operand of `&` must be a local place expression"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_mutable_borrow_of_immutable_local() {
        let root = make_temp_package(
            "typed_immutable_mut_borrow",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "fn main() -> Int:\n    let x = 1\n    let y = &mut x\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("mutable borrow of immutable local should fail");
        assert!(
            err.contains("cannot mutably borrow immutable local `x`"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_mutable_borrow_while_shared_borrow_active() {
        let root = make_temp_package(
            "typed_borrow_conflict",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "fn main() -> Int:\n    let mut x = 1\n    let a = &x\n    let b = &mut x\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("conflicting borrows should fail");
        assert!(
            err.contains("cannot mutably borrow `x` while it is already borrowed"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_accepts_borrowed_array_slices_as_views() {
        let std_dep = repo_root().join("std").to_string_lossy().replace('\\', "/");
        let root = make_temp_package(
            "typed_borrowed_slice_views",
            "app",
            &[("std", std_dep.as_str())],
            &[
                (
                    "src/shelf.arc",
                    concat!(
                        "import std.collections.array\n",
                        "fn main() -> Int:\n",
                        "    let mut read_xs = std.collections.array.new[Int] :: 4, 0 :: call\n",
                        "    read_xs[0] = 1\n",
                        "    read_xs[1] = 2\n",
                        "    read_xs[2] = 3\n",
                        "    read_xs[3] = 4\n",
                        "    let view = &read_xs[1..3]\n",
                        "    let mut edit_xs = std.collections.array.new[Int] :: 2, 0 :: call\n",
                        "    edit_xs[0] = 5\n",
                        "    edit_xs[1] = 6\n",
                        "    let mut edit = &mut edit_xs[0..2]\n",
                        "    edit :: 1, 9 :: set\n",
                        "    return (view :: :: len) + (edit :: :: len)\n",
                    ),
                ),
                ("src/types.arc", ""),
            ],
        );

        check_path(&root).expect("borrowed array slices should type-check as views");
        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_mutable_string_slice_borrow() {
        let root = make_temp_package(
            "typed_mut_borrow_string_slice",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "fn main() -> Int:\n    let mut text = \"hello\"\n    let x = &mut text[1..4]\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("mutable string slice borrow should fail");
        assert!(
            err.contains("string slices are read-only; `&mut x[a..b]` is not allowed"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_borrowed_list_slices() {
        let root = make_temp_package(
            "typed_list_slice_borrow",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "fn main() -> Int:\n    let xs = [1, 2, 3]\n    let view = &xs[0..2]\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("list slice borrow should fail");
        assert!(
            err.contains("borrowed slices require contiguous backing; `List` is not supported"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_assignment_while_local_borrowed() {
        let root = make_temp_package(
            "typed_assign_while_borrowed",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "fn main() -> Int:\n    let mut x = 1\n    let a = &x\n    x = 2\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("assignment while borrowed should fail");
        assert!(
            err.contains("cannot assign to local `x` while it is borrowed"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_direct_access_while_mutably_borrowed() {
        let root = make_temp_package(
            "typed_mut_borrow_direct_access",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "fn main() -> Int:\n    let mut x = 1\n    let a = &mut x\n    let y = x + 1\n    return y\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("direct access while mutably borrowed should fail");
        assert!(
            err.contains("cannot access local `x` directly while it is mutably borrowed"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_allows_assignment_to_edit_param() {
        let root = make_temp_package(
            "typed_edit_param_assign",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "fn bump(edit n: Int):\n    n = n + 1\nfn main() -> Int:\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let summary = check_path(&root).expect("assignment to edit param should check");
        assert_eq!(summary.package_count, 1);
        assert_eq!(summary.module_count, 2);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_untied_return_lifetime() {
        let root = make_temp_package(
            "typed_untied_return_lifetime",
            "app",
            &[],
            &[
                ("src/shelf.arc", "fn bad['a]() -> &'a Int:\n    return 1\n"),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("untied return lifetime should fail");
        assert!(
            err.contains("return lifetime `'a` must be tied to an input parameter"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_accepts_structured_where_predicates() {
        let root = make_temp_package(
            "typed_where_ok",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "trait Iterator[I]:\n    type Item\nfn main['a, I, U, where Iterator[I], Iterator[I].Item = U, U: 'a]() -> Int:\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let summary = check_path(&root).expect("structured where predicates should check");
        assert_eq!(summary.package_count, 1);
        assert!(summary.module_count >= 2);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_non_trait_where_predicates() {
        let root = make_temp_package(
            "typed_where_non_trait",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "record NotTrait:\n    value: Int\nfn main[T, where NotTrait]() -> Int:\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("non-trait where predicate should fail");
        assert!(err.contains("does not resolve to a valid trait"), "{err}");

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_malformed_projection_equality_where_predicates() {
        let root = make_temp_package(
            "typed_where_bad_projection",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "trait Iterator[I]:\n    type Item\nfn main[I, U, where Iterator[I] = U]() -> Int:\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("bad projection equality should fail");
        assert!(
            err.contains("unsupported top-level syntax")
                || err.contains("projection-equality predicate `Iterator[I]`"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_undeclared_outlives_where_lifetimes() {
        let root = make_temp_package(
            "typed_where_bad_outlives",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "fn main['a, T, where T: 'b]() -> Int:\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("undeclared outlives lifetime should fail");
        assert!(
            err.contains("undeclared lifetime `'b` in where predicate `T: 'b`"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_projection_equality_unknown_assoc_types() {
        let root = make_temp_package(
            "typed_where_unknown_assoc",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "trait Iterator[I]:\n    type Item\nfn main[I, U, where Iterator[I].Missing = U]() -> Int:\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err =
            check_path(&root).expect_err("projection equality with unknown assoc type should fail");
        assert!(
            err.contains("does not declare associated type `Missing`"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_accepts_projection_equality_where_predicates() {
        let root = make_temp_package(
            "typed_where_projection_ok",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "trait Iterator[I]:\n    type Item\nfn main[I, U, where Iterator[I].Item = U]() -> Int:\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        check_path(&root).expect("projection equality with known assoc type should check");

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_accepts_declared_outlives_where_predicates() {
        let root = make_temp_package(
            "typed_where_outlives_ok",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "fn main['a, 'b, T, where 'a: 'b, T: 'a]() -> Int:\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        check_path(&root).expect("declared outlives predicates should check");

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_impls_missing_trait_where_requirements() {
        let root = make_temp_package(
            "typed_where_missing_impl_req",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    concat!(
                        "trait Eq[T]:\n",
                        "    fn ok(read self: T) -> Int:\n",
                        "        return 0\n",
                        "trait Ord[T, where Eq[T]]:\n",
                        "    fn cmp(read self: T) -> Int:\n",
                        "        return 0\n",
                        "impl Ord[Int] for Int:\n",
                        "    fn cmp(read self: Int) -> Int:\n",
                        "        return 0\n",
                        "fn main() -> Int:\n",
                        "    return 0\n",
                    ),
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("missing required trait impl should fail");
        assert!(
            err.contains("impl requires satisfying where-bound `Eq[Int]`"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_accepts_impls_with_trait_where_requirements_satisfied() {
        let root = make_temp_package(
            "typed_where_impl_req_ok",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    concat!(
                        "trait Eq[T]:\n",
                        "    fn ok(read self: T) -> Int:\n",
                        "        return 0\n",
                        "trait Ord[T, where Eq[T]]:\n",
                        "    fn cmp(read self: T) -> Int:\n",
                        "        return 0\n",
                        "impl Eq[Int] for Int:\n",
                        "    fn ok(read self: Int) -> Int:\n",
                        "        return 0\n",
                        "impl Ord[Int] for Int:\n",
                        "    fn cmp(read self: Int) -> Int:\n",
                        "        return 0\n",
                        "fn main() -> Int:\n",
                        "    return 0\n",
                    ),
                ),
                ("src/types.arc", ""),
            ],
        );

        check_path(&root).expect("satisfied trait where requirements should check");

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_ambiguous_concrete_bare_methods() {
        let root = make_temp_package(
            "ambiguous_bare_method_concrete",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    concat!(
                        "import types\n",
                        "use types.Counter\n",
                        "fn main() -> Int:\n",
                        "    let counter = Counter :: value = 1 :: call\n",
                        "    return counter :: :: tap\n",
                    ),
                ),
                ("src/types.arc", "export record Counter:\n    value: Int\n"),
                (
                    "src/left.arc",
                    concat!(
                        "import types\n",
                        "use types.Counter\n",
                        "impl Counter:\n",
                        "    fn tap(read self: Counter) -> Int:\n",
                        "        return self.value + 1\n",
                    ),
                ),
                (
                    "src/right.arc",
                    concat!(
                        "import types\n",
                        "use types.Counter\n",
                        "impl Counter:\n",
                        "    fn tap(read self: Counter) -> Int:\n",
                        "        return self.value + 2\n",
                    ),
                ),
            ],
        );

        let err = check_path(&root).expect_err("ambiguous concrete bare method should fail");
        assert!(
            err.contains("bare-method qualifier `tap` on `ambiguous_bare_method_concrete.types.Counter` is ambiguous"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_ignores_unrelated_workspace_methods_for_bare_resolution() {
        let root = make_temp_workspace(
            "unrelated_bare_method_scope",
            &["app", "core", "extra"],
            &[
                (
                    "app/book.toml",
                    "name = \"app\"\nkind = \"app\"\n\n[deps]\ncore = { path = \"../core\" }\n",
                ),
                (
                    "app/src/shelf.arc",
                    concat!(
                        "import core.types\n",
                        "use core.types.Counter\n",
                        "fn main() -> Int:\n",
                        "    let counter = Counter :: value = 1 :: call\n",
                        "    return counter :: :: tap\n",
                    ),
                ),
                ("app/src/types.arc", ""),
                ("core/book.toml", "name = \"core\"\nkind = \"lib\"\n"),
                (
                    "core/src/book.arc",
                    "import types\nuse types.Counter\nimpl Counter:\n    fn tap(read self: Counter) -> Int:\n        return self.value + 1\n",
                ),
                (
                    "core/src/types.arc",
                    "export record Counter:\n    value: Int\n",
                ),
                ("extra/book.toml", "name = \"extra\"\nkind = \"lib\"\n"),
                (
                    "extra/src/book.arc",
                    "import types\nuse types.Counter\nimpl Counter:\n    fn tap(read self: Counter) -> Int:\n        return self.value + 99\n",
                ),
                (
                    "extra/src/types.arc",
                    "export record Counter:\n    value: Int\n",
                ),
            ],
        );

        check_path(&root.join("app"))
            .expect("unrelated workspace package should not affect bare methods");

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_duplicate_top_level_symbols() {
        let root = make_temp_package(
            "typed_duplicate_top_level_symbol",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "export fn mouse_in_window(read win: Window) -> Bool:\n    return false\nexport fn mouse_in_window(read win: Window) -> Bool:\n    return true\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("duplicate top-level symbol should fail");
        assert!(err.contains("duplicate symbol `mouse_in_window`"), "{err}");

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_duplicate_directive_bindings() {
        let root = make_temp_workspace(
            "typed_duplicate_directive_binding",
            &["app", "io", "text"],
            &[
                (
                    "app/book.toml",
                    "name = \"app\"\nkind = \"app\"\n\n[deps]\nio = { path = \"../io\" }\ntext = { path = \"../text\" }\n",
                ),
                (
                    "app/src/shelf.arc",
                    "use io as io\nuse text as io\nfn main() -> Int:\n    return 0\n",
                ),
                ("app/src/types.arc", ""),
                ("io/book.toml", "name = \"io\"\nkind = \"lib\"\n"),
                (
                    "io/src/book.arc",
                    "export fn print() -> Int:\n    return 0\n",
                ),
                ("io/src/types.arc", ""),
                ("text/book.toml", "name = \"text\"\nkind = \"lib\"\n"),
                (
                    "text/src/book.arc",
                    "export fn print() -> Int:\n    return 0\n",
                ),
                ("text/src/types.arc", ""),
            ],
        );

        let err =
            check_path(&root.join("app")).expect_err("duplicate directive binding should fail");
        assert!(err.contains("duplicate binding `io`"), "{err}");

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_return_borrow_of_local() {
        let root = make_temp_package(
            "typed_return_local_borrow",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "fn bad['a](read value: &'a Int) -> &'a Int:\n    let x = 1\n    return &x\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("return borrow of local should fail");
        assert!(
            err.contains(
                "returned reference must be tied to input lifetimes; local `x` does not live long enough"
            ),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_use_after_take_move() {
        let root = make_temp_package(
            "typed_take_use_after_move",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "fn consume(take value: Str):\n    return\nfn main() -> Int:\n    let s = \"hi\"\n    consume :: s :: call\n    s :: :: std.io.print\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("use after take-move should fail");
        assert!(err.contains("use of moved local `s`"), "{err}");

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_take_move_while_borrowed() {
        let root = make_temp_package(
            "typed_take_while_borrowed",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "fn consume(take value: Str):\n    return\nfn main() -> Int:\n    let s = \"hi\"\n    let r = &s\n    consume :: s :: call\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("moving borrowed local should fail");
        assert!(
            err.contains("cannot move local `s` while it is borrowed"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_allows_copy_value_after_take_param_call() {
        let root = make_temp_package(
            "typed_take_copy_ok",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "fn consume(take value: Int):\n    return\nfn main() -> Int:\n    let x = 1\n    consume :: x :: call\n    return x\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let summary = check_path(&root).expect("copy value after take call should check");
        assert_eq!(summary.package_count, 1);
        assert_eq!(summary.module_count, 2);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_non_place_edit_param_call() {
        let root = make_temp_package(
            "typed_edit_non_place",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "fn bump(edit n: Int):\n    n = n + 1\nfn main() -> Int:\n    bump :: (1 + 2) :: call\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("edit call on non-place should fail");
        assert!(
            err.contains("argument for edit parameter `n` must be a local place expression"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_immutable_local_edit_param_call() {
        let root = make_temp_package(
            "typed_edit_immutable_local",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "fn bump(edit n: Int):\n    n = n + 1\nfn main() -> Int:\n    let x = 1\n    bump :: x :: call\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("edit call on immutable local should fail");
        assert!(
            err.contains("cannot pass immutable local `x` to edit parameter `n`"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_method_style_take_use_after_move() {
        let root = make_temp_package(
            "typed_method_take_move",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "record Bag:\n    n: Int\nimpl Bag:\n    fn push(edit self: Bag, take value: Str):\n        self.n = self.n + 1\nfn main() -> Int:\n    let bag = Bag :: n = 0 :: call\n    let s = \"hi\"\n    bag :: s :: push\n    let t = s\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("method-style take move should fail");
        assert!(err.contains("use of moved local `s`"), "{err}");

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_window_use_after_close() {
        let std_dep = repo_root().join("std").to_string_lossy().replace('\\', "/");
        let root = make_temp_package(
            "typed_window_use_after_close",
            "app",
            &[("std", std_dep.as_str())],
            &[
                (
                    "src/shelf.arc",
                    "import std.window\nuse std.window.Window\nfn bad(take win: Window) -> Int:\n    std.window.close :: win :: call\n    let alive = std.window.alive :: win :: call\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("window use after close should fail");
        assert!(err.contains("use of moved local `win`"), "{err}");

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_stream_use_after_close() {
        let std_dep = repo_root().join("std").to_string_lossy().replace('\\', "/");
        let root = make_temp_package(
            "typed_stream_use_after_close",
            "app",
            &[("std", std_dep.as_str())],
            &[
                (
                    "src/shelf.arc",
                    "import std.fs\nuse std.fs.FileStream\nfn bad(take stream: FileStream) -> Int:\n    std.fs.stream_close :: stream :: call\n    let done = std.fs.stream_eof :: stream :: call\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("stream use after close should fail");
        assert!(err.contains("use of moved local `stream`"), "{err}");

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_accepts_opaque_type_outside_std() {
        let root = make_temp_package(
            "opaque_type_outside_std",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "export opaque type Token as move, boundary_safe\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        check_path(&root).expect("opaque types outside std should check");

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_accepts_grimoire_opaque_family_lang_item() {
        let root = make_temp_package(
            "desktop",
            "lib",
            &[],
            &[
                (
                    "src/types.arc",
                    concat!(
                        "export opaque type Window as move, boundary_unsafe\n",
                        "lang window_handle = Window\n",
                    ),
                ),
                ("src/book.arc", "reexport desktop.types\n"),
            ],
        );

        check_path(&root).expect("grimoire opaque family binding should check");

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_duplicate_opaque_family_lang_item_in_package() {
        let root = make_temp_package(
            "desktop",
            "lib",
            &[],
            &[
                (
                    "src/types.arc",
                    concat!(
                        "export opaque type Window as move, boundary_unsafe\n",
                        "lang window_handle = Window\n",
                    ),
                ),
                (
                    "src/extra.arc",
                    concat!(
                        "export opaque type AltWindow as move, boundary_unsafe\n",
                        "lang window_handle = AltWindow\n",
                    ),
                ),
                (
                    "src/book.arc",
                    "reexport desktop.types\nreexport desktop.extra\n",
                ),
            ],
        );

        let err = check_path(&root).expect_err("duplicate opaque family binding should fail");
        assert!(
            err.contains("opaque family lang item `window_handle` is declared more than once"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_accepts_std_owned_opaque_type_impl_target() {
        let root = make_temp_package(
            "std",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "export opaque type Token[T] as move, boundary_safe\nimpl[T] Token[T]:\n    fn id(read self: Token[T]) -> Int:\n        return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        check_path(&root).expect("std opaque type impl target should check");

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_std_opaque_type_builtin_name_collision() {
        let root = make_temp_package(
            "std",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "export opaque type Int as move, boundary_safe\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("builtin-name opaque type should fail");
        assert!(
            err.contains("opaque type `Int` conflicts with reserved builtin type name"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_opaque_type_constructor_use() {
        let std_dep = repo_root().join("std").to_string_lossy().replace('\\', "/");
        let root = make_temp_package(
            "opaque_type_constructor_use",
            "app",
            &[("std", std_dep.as_str())],
            &[
                (
                    "src/shelf.arc",
                    "use std.window.Window\nfn bad() -> Int:\n    let win = Window :: :: call\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("opaque constructors should fail");
        assert!(
            err.contains("opaque type `Window` is not constructible"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_boundary_unsafe_std_opaque_type() {
        let std_dep = repo_root().join("std").to_string_lossy().replace('\\', "/");
        let root = make_temp_package(
            "opaque_type_boundary_contract",
            "app",
            &[("std", std_dep.as_str())],
            &[
                (
                    "src/shelf.arc",
                    "use std.window.Window\n#boundary[target = \"lua\"]\nexport fn bad(read win: Window) -> Int:\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("boundary-unsafe opaque type should fail");
        assert!(
            err.contains("type `Window` is not boundary-safe for target `lua`"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_accepts_return_lifetime_tied_to_param() {
        let root = make_temp_package(
            "typed_tied_return_lifetime",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "fn keep['a](read value: &'a Int) -> &'a Int:\n    return value\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let summary = check_path(&root).expect("tied return lifetime should check");
        assert_eq!(summary.package_count, 1);
        assert_eq!(summary.module_count, 2);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_handles_local_member_field_access() {
        let root = make_temp_package(
            "local_member_access",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "record Box:\n    value: Int\nfn main() -> Int:\n    let item = Box :: value = 1 :: call\n    return item.value\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let summary = check_path(&root).expect("local member access should check");
        assert_eq!(summary.package_count, 1);
        assert_eq!(summary.module_count, 2);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_sources_accepts_object_owner_activation_flow() {
        let summary = check_sources(
            [concat!(
                "obj Counter:\n",
                "    value: Int\n",
                "\n",
                "create Session [Counter] scope-exit:\n",
                "    done: when Counter.value > 0 hold [Counter]\n",
                "\n",
                "Session\n",
                "Counter\n",
                "fn main() -> Int:\n",
                "    let active = Session :: :: call\n",
                "    Counter.value = 1\n",
                "    return Counter.value\n",
            )]
            .iter()
            .copied(),
        )
        .expect("object/owner flow should check");
        assert!(summary.symbol_count >= 3);
    }

    #[test]
    fn check_sources_accepts_object_only_attached_owner_flow() {
        let summary = check_sources(
            [concat!(
                "obj Counter:\n",
                "    value: Int\n",
                "\n",
                "create Session [Counter] scope-exit:\n",
                "    done: when Counter.value > 0 hold [Counter]\n",
                "\n",
                "Counter\n",
                "fn bump() -> Int:\n",
                "    Counter.value += 1\n",
                "    return Counter.value\n",
                "\n",
                "Session\n",
                "Counter\n",
                "fn main() -> Int:\n",
                "    let active = Session :: :: call\n",
                "    Counter.value = 4\n",
                "    return bump :: :: call\n",
            )]
            .iter()
            .copied(),
        )
        .expect("object-only attached owner flow should check");
        assert!(summary.symbol_count >= 3);
    }

    #[test]
    fn check_sources_rejects_owner_without_scope_exit_clause() {
        let root = make_temp_package(
            "owner_missing_exit",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    concat!(
                        "obj Counter:\n",
                        "    value: Int\n",
                        "\n",
                        "create Session [Counter] scope-exit:\n",
                        "\n",
                        "fn main() -> Int:\n",
                        "    return 0\n",
                    ),
                ),
                ("src/types.arc", ""),
            ],
        );
        let err = check_path(&root).expect_err("owner without exit should fail");
        assert!(
            err.contains("must declare at least one scope-exit"),
            "{err}"
        );
        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_sources_rejects_non_bool_owner_exit_condition() {
        let root = make_temp_package(
            "owner_non_bool_exit",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    concat!(
                        "obj Counter:\n",
                        "    value: Int\n",
                        "\n",
                        "create Session [Counter] scope-exit:\n",
                        "    exit when 1\n",
                        "\n",
                        "fn main() -> Int:\n",
                        "    return 0\n",
                    ),
                ),
                ("src/types.arc", ""),
            ],
        );
        let err = check_path(&root).expect_err("non-bool owner exit condition should fail");
        assert!(
            err.contains("owner exit condition requires Bool, found Int"),
            "{err}"
        );
        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_accepts_object_owner_lifecycle_conformance_package() {
        let summary = check_path(
            &repo_root()
                .join("conformance")
                .join("fixtures")
                .join("object_owner_lifecycle_workspace")
                .join("app"),
        )
        .expect("lifecycle conformance fixture should check");
        assert!(summary.package_count >= 1);
        assert!(summary.module_count >= 2);
    }

    #[test]
    fn check_path_rejects_owner_invalid_lifecycle_hook_package() {
        let err = check_path(
            &repo_root()
                .join("conformance")
                .join("check_parity_packages")
                .join("owner_invalid_lifecycle_hook"),
        )
        .expect_err("invalid lifecycle hook fixture should fail");
        assert!(err.contains("must take `edit self`"), "{err}");
    }

    #[test]
    fn check_path_rejects_owner_wrong_context_type_package() {
        let err = check_path(
            &repo_root()
                .join("conformance")
                .join("check_parity_packages")
                .join("owner_wrong_context_type"),
        )
        .expect_err("wrong context type fixture should fail");
        assert!(err.contains("expects context"), "{err}");
    }

    #[test]
    fn check_sources_rejects_invalid_object_lifecycle_hook_signature() {
        let root = make_temp_package(
            "owner_invalid_hook",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    concat!(
                        "obj Counter:\n",
                        "    value: Int\n",
                        "    fn init(read self: Self):\n",
                        "        return\n",
                        "\n",
                        "create Session [Counter] scope-exit:\n",
                        "    done: when false hold [Counter]\n",
                        "\n",
                        "fn main() -> Int:\n",
                        "    return 0\n",
                    ),
                ),
                ("src/types.arc", ""),
            ],
        );
        let err = check_path(&root).expect_err("invalid lifecycle hook should fail");
        assert!(err.contains("must take `edit self`"), "{err}");
        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_sources_rejects_owner_activation_with_wrong_context_type() {
        let root = make_temp_package(
            "owner_wrong_context",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    concat!(
                        "obj SessionCtx:\n",
                        "    base: Int\n",
                        "\n",
                        "obj Counter:\n",
                        "    value: Int\n",
                        "    fn init(edit self: Self, read ctx: SessionCtx):\n",
                        "        self.value = ctx.base\n",
                        "\n",
                        "create Session [Counter] scope-exit:\n",
                        "    done: when Counter.value > 10 hold [Counter]\n",
                        "\n",
                        "Session\n",
                        "Counter\n",
                        "fn main() -> Int:\n",
                        "    Session :: 1 :: call\n",
                        "    return 0\n",
                    ),
                ),
                ("src/types.arc", ""),
            ],
        );
        let err = check_path(&root).expect_err("wrong owner activation context should fail");
        assert!(err.contains("expects context"), "{err}");
        assert!(err.contains("Int"), "{err}");
        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_handles_result_variant_constructor_package() {
        let std_dep = repo_root().join("std").to_string_lossy().replace('\\', "/");
        let root = make_temp_package(
            "result_variant_positive",
            "app",
            &[("std", std_dep.as_str())],
            &[
                (
                    "src/shelf.arc",
                    "import std.result\nuse std.result.Result\nfn parse(flag: Bool) -> Result[Int, Str]:\n    if flag:\n        return Result.Ok[Int, Str] :: 1 :: call\n    return Result.Err[Int, Str] :: \"bad\" :: call\nfn main() -> Int:\n    let parsed = parse :: true :: call\n    return match parsed:\n        Result.Ok(value) => value\n        Result.Err(_) => 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let summary = check_path(&root).expect("result variant package should resolve");
        assert_eq!(summary.package_count, 2);
        assert!(summary.module_count >= 2);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_handles_mixed_chain_package() {
        let root = make_temp_package(
            "mixed_chain_positive",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "import stage\nfn main() -> Int:\n    let seed = 1\n    let score = forward :=> stage.seed with (seed) => stage.inc <= stage.dec <= stage.emit\n    return score\n",
                ),
                (
                    "src/stage.arc",
                    "export fn seed(seed: Int) -> Int:\n    return seed\nexport fn inc(value: Int) -> Int:\n    return value + 1\nexport fn dec(value: Int) -> Int:\n    return value - 1\nexport fn emit(value: Int) -> Int:\n    return value\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let summary = check_path(&root).expect("mixed chain package should resolve");
        assert_eq!(summary.package_count, 1);
        assert!(summary.module_count >= 3);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_handles_bound_chain_workspace() {
        let root = make_temp_workspace(
            "bound_chain_workspace",
            &["app", "core"],
            &[
                (
                    "app/book.toml",
                    "name = \"app\"\nkind = \"app\"\n\n[deps]\ncore = { path = \"../core\" }\n",
                ),
                (
                    "app/src/shelf.arc",
                    "import core\nfn main() -> Int:\n    let seed = 1\n    let score = forward :=> core.seed with (seed) => core.inc <= core.dec <= core.emit\n    return score\n",
                ),
                ("app/src/types.arc", ""),
                ("core/book.toml", "name = \"core\"\nkind = \"lib\"\n"),
                (
                    "core/src/book.arc",
                    "export fn seed(seed: Int) -> Int:\n    return seed\nexport fn inc(value: Int) -> Int:\n    return value + 1\nexport fn dec(value: Int) -> Int:\n    return value - 1\nexport fn emit(value: Int) -> Int:\n    return value\n",
                ),
                ("core/src/types.arc", ""),
            ],
        );

        let summary = check_path(&root).expect("bound chain workspace should resolve");
        assert_eq!(summary.package_count, 2);
        assert!(summary.module_count >= 4);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_filters_only_forewords_for_current_target() {
        let root = make_temp_package(
            "only_filter_app",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "#only[os = \"definitely_not_host\"]\nfn skipped() -> MissingType:\n    return 0\nfn main() -> Int:\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let summary = check_path(&root).expect("#only should exclude non-matching declarations");
        assert_eq!(summary.package_count, 1);
        assert_eq!(summary.module_count, 2);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn load_workspace_hir_exposes_package_summaries() {
        let root = make_temp_workspace(
            "workspace_hir_summary",
            &["app", "core"],
            &[
                (
                    "app/book.toml",
                    "name = \"app\"\nkind = \"app\"\n\n[deps]\ncore = { path = \"../core\" }\n",
                ),
                (
                    "app/src/shelf.arc",
                    "import core\nfn main() -> Int:\n    return core.id :: 1 :: call\n",
                ),
                ("app/src/types.arc", ""),
                ("core/book.toml", "name = \"core\"\nkind = \"lib\"\n"),
                (
                    "core/src/book.arc",
                    "export fn id(x: Int) -> Int:\n    return x\n",
                ),
                ("core/src/types.arc", ""),
            ],
        );

        let workspace = load_workspace_hir(&root).expect("workspace hir should load");
        assert!(workspace.package("app").is_some());
        assert!(workspace.package("core").is_some());
        assert!(
            workspace
                .package("app")
                .expect("app package should exist")
                .summary
                .dependency_edges
                .iter()
                .any(|edge| edge.target_path == vec!["core".to_string()])
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_sources_rejects_headed_region_semantic_violations() {
        for (source, expected) in [
            (
                concat!(
                    "record Widget:\n",
                    "    value: Int\n",
                    "    maybe: Option[Int]\n",
                    "fn main() -> Int:\n",
                    "    let built = construct yield Widget -return 0\n",
                    "        maybe = Option.None[Int] :: :: call\n",
                    "    return 0\n",
                ),
                "missing required field `value`",
            ),
            (
                concat!(
                    "record Widget:\n",
                    "    value: Int\n",
                    "    maybe: Option[Int]\n",
                    "enum Option[T]:\n",
                    "    Some(T)\n",
                    "    None\n",
                    "record Other:\n",
                    "    value: Str\n",
                    "fn main() -> Int:\n",
                    "    let base = Other :: value = \"x\" :: call\n",
                    "    let built = record yield Widget from base -return 0\n",
                    "        maybe = Option.None[Int] :: :: call\n",
                    "    return 0\n",
                ),
                "record base field `value` has incompatible type `Str` for target `Widget` field `Int`",
            ),
            (
                concat!(
                    "record Widget:\n",
                    "    value: Int\n",
                    "fn main() -> Int:\n",
                    "    let base = 1\n",
                    "    let built = record yield Widget from base -return 0\n",
                    "        value = 1\n",
                    "    return 0\n",
                ),
                "record base must have a known record type in v1",
            ),
            (
                concat!(
                    "record Widget:\n",
                    "    value: Int\n",
                    "    maybe: Option[Int]\n",
                    "enum Option[T]:\n",
                    "    Some(T)\n",
                    "    None\n",
                    "fn main() -> Int:\n",
                    "    let built = record yield Widget -return 0\n",
                    "        payload = 1\n",
                    "    return 0\n",
                ),
                "`record` does not accept `payload = ...` contributions",
            ),
            (
                concat!(
                    "enum Result[T, E]:\n",
                    "    Ok(T)\n",
                    "    Err(E)\n",
                    "fn main() -> Int:\n",
                    "    bind -return 0\n",
                    "        let value = Result.Err[Int, Str] :: \"no\" :: call -preserve\n",
                    "    return 0\n",
                ),
                "`bind -preserve` is only valid on `name = gate` lines",
            ),
            (
                concat!(
                    "fn main() -> Int:\n",
                    "    bind -continue\n",
                    "        require false\n",
                    "    return 0\n",
                ),
                "`bind -continue` is only valid inside a loop",
            ),
            (
                concat!(
                    "enum Result[T, E]:\n",
                    "    Ok(T)\n",
                    "    Err(E)\n",
                    "fn main() -> Int:\n",
                    "    while true:\n",
                    "        bind -continue\n",
                    "            let value = Result.Ok[Int, Str] :: 1 :: call\n",
                    "    return 0\n",
                ),
                "`bind -continue` is only valid on `require <expr>` lines",
            ),
            (
                concat!(
                    "fn main() -> Int:\n",
                    "    recycle\n",
                    "        true\n",
                    "    return 0\n",
                ),
                "recycle requires a default modifier in v1",
            ),
            (
                concat!(
                    "fn main() -> Int:\n",
                    "    recycle -return 0\n",
                    "        false -return\n",
                    "    return 0\n",
                ),
                "bare `-return` in recycle requires Result failure",
            ),
            (
                concat!(
                    "fn main() -> Int:\n",
                    "    recycle -return 0\n",
                    "        false -done\n",
                    "    return 0\n",
                ),
                "named recycle exit `done` is not active on this path",
            ),
            (
                concat!(
                    "obj Counter:\n",
                    "    value: Int\n",
                    "create Session [Counter] scope-exit:\n",
                    "    done: when false hold [Counter]\n",
                    "fn helper() -> Int:\n",
                    "    recycle -done\n",
                    "        false\n",
                    "    return 1\n",
                    "fn main() -> Int:\n",
                    "    return 0\n",
                ),
                "named recycle exit `done` is not active on this path",
            ),
            (
                concat!(
                    "fn main() -> Int:\n",
                    "    recycle -return 0\n",
                    "        false -continue\n",
                    "    return 0\n",
                ),
                "`break` and `continue` recycle exits are only valid inside loops",
            ),
            (
                concat!(
                    "Memory arena:cache -alloc\n",
                    "    recycle = free_list\n",
                    "fn main() -> Int:\n",
                    "    return 0\n",
                ),
                "memory detail `recycle` is not supported for family `arena`",
            ),
            (
                concat!(
                    "Memory arena:cache -recycle\n",
                    "    capacity = 4\n",
                    "fn main() -> Int:\n",
                    "    return 0\n",
                ),
                "memory modifier `-recycle` is not supported for family `arena`",
            ),
            (
                concat!(
                    "record Inner:\n",
                    "    value: Int\n",
                    "record Outer:\n",
                    "    inner: Inner\n",
                    "fn main() -> Int:\n",
                    "    let outer = construct yield Outer -return 0\n",
                    "        inner = construct yield Inner -return 0\n",
                    "            value = 1\n",
                    "    return 0\n",
                ),
                "headed regions cannot nest inside another headed region in v1",
            ),
            (
                concat!(
                    "record Widget:\n",
                    "    value: Int\n",
                    "record Other:\n",
                    "    value: Int\n",
                    "fn main() -> Int:\n",
                    "    let target = Other :: value = 0 :: call\n",
                    "    construct place Widget -> target -return 0\n",
                    "        value = 1\n",
                    "    return 0\n",
                ),
                "construct place target type `headed_region_semantic_violation.Other` does not match constructor result type `headed_region_semantic_violation.Widget`",
            ),
            (
                concat!(
                    "record Widget:\n",
                    "    value: Int\n",
                    "record Other:\n",
                    "    value: Int\n",
                    "fn main() -> Int:\n",
                    "    let target = Other :: value = 0 :: call\n",
                    "    record place Widget -> target -return 0\n",
                    "        value = 1\n",
                    "    return 0\n",
                ),
                "record place target type `headed_region_semantic_violation.Other` does not match record result type `headed_region_semantic_violation.Widget`",
            ),
            (
                concat!(
                    "record Widget:\n",
                    "    value: Int\n",
                    "fn main() -> Int:\n",
                    "    let built = construct yield Widget -break\n",
                    "        value = 1\n",
                    "    return 0\n",
                ),
                "`construct` does not support `-break` modifiers in v1",
            ),
            (
                concat!(
                    "record Widget:\n",
                    "    value: Int\n",
                    "fn main() -> Int:\n",
                    "    let built = construct yield Widget -return 0\n",
                    "        value = true\n",
                    "    return 0\n",
                ),
                "construct contribution `value` has type `Bool` but target `Widget.value` expects `Int`",
            ),
            (
                concat!(
                    "record Widget:\n",
                    "    value: Int\n",
                    "enum Option[T]:\n",
                    "    Some(T)\n",
                    "    None\n",
                    "fn main() -> Int:\n",
                    "    let built = construct yield Widget -return 0\n",
                    "        value = Option.None[Int] :: :: call -default false\n",
                    "    return 0\n",
                ),
                "`construct -default` fallback for `value` must have type `Int`",
            ),
            (
                concat!(
                    "enum Option[T]:\n",
                    "    Some(T)\n",
                    "    None\n",
                    "fn main() -> Int:\n",
                    "    bind -return 0\n",
                    "        missing = Option.Some[Int] :: 1 :: call\n",
                    "    return 0\n",
                ),
                "unresolved value reference `missing` in assignment target",
            ),
            (
                concat!(
                    "enum Option[T]:\n",
                    "    Some(T)\n",
                    "    None\n",
                    "fn main() -> Int:\n",
                    "    recycle -return 0\n",
                    "        missing = Option.Some[Int] :: 1 :: call\n",
                    "    return 0\n",
                ),
                "unresolved value reference `missing` in assignment target",
            ),
            (
                concat!(
                    "enum Option[T]:\n",
                    "    Some(T)\n",
                    "    None\n",
                    "fn main() -> Int:\n",
                    "    let mut value = 1\n",
                    "    bind -return 0\n",
                    "        value = Option.None[Int] :: :: call -replace \"x\"\n",
                    "    return value\n",
                ),
                "`bind -replace` fallback for `value` must have type `Int`",
            ),
            (
                concat!(
                    "enum Option[T]:\n",
                    "    Some(T)\n",
                    "    None\n",
                    "fn main() -> Int:\n",
                    "    bind -return 0\n",
                    "        let value = Option.None[Int] :: :: call -default \"x\"\n",
                    "    return value\n",
                ),
                "`bind -default` fallback for `value` must have type `Int`",
            ),
            (
                concat!(
                    "enum Option[T]:\n",
                    "    Some(T)\n",
                    "    None\n",
                    "fn main() -> Int:\n",
                    "    let mut value = \"x\"\n",
                    "    bind -return 0\n",
                    "        value = Option.Some[Int] :: 1 :: call -preserve\n",
                    "    return 0\n",
                ),
                "`bind -preserve` payload for `value` must have type `Str`",
            ),
            (
                concat!(
                    "enum Option[T]:\n",
                    "    Some(T)\n",
                    "    None\n",
                    "fn main() -> Int:\n",
                    "    bind -return \"x\"\n",
                    "        let value = Option.None[Int] :: :: call\n",
                    "    return 0\n",
                ),
                "`bind -return` payload must have type `Int`",
            ),
            (
                concat!(
                    "fn main() -> Int:\n",
                    "    recycle -return \"x\"\n",
                    "        false\n",
                    "    return 0\n",
                ),
                "`recycle -return` payload must have type `Int`",
            ),
            (
                concat!(
                    "record Widget:\n",
                    "    value: Int\n",
                    "fn main() -> Int:\n",
                    "    let built = construct yield Widget -return \"x\"\n",
                    "        value = 1\n",
                    "    return 0\n",
                ),
                "`construct -return` payload must have type `Int`",
            ),
            (
                concat!(
                    "enum Result[T, E]:\n",
                    "    Ok(T)\n",
                    "    Err(E)\n",
                    "fn main() -> Int:\n",
                    "    bind -return\n",
                    "        let value = Result.Err[Int, Str] :: \"x\" :: call\n",
                    "    return 0\n",
                ),
                "bare `bind -return` failure must have type `Int`",
            ),
        ] {
            let root = make_temp_package(
                "headed_region_semantic_violation",
                "app",
                &[],
                &[("src/shelf.arc", source), ("src/types.arc", "")],
            );
            let err = check_path(&root).expect_err("fixture should fail");
            assert!(err.contains(expected), "{expected}: {err}");
            fs::remove_dir_all(root).expect("cleanup should succeed");
        }
    }

    #[test]
    fn check_path_accepts_same_region_headed_bindings_and_matching_construct_place() {
        let root = make_temp_package(
            "headed_region_same_scope_positive",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    concat!(
                        "record Widget:\n",
                        "    value: Int\n",
                        "enum Option[T]:\n",
                        "    Some(T)\n",
                        "    None\n",
                        "fn main() -> Int:\n",
                        "    bind -return 0\n",
                        "        let value = Option.Some[Int] :: 1 :: call\n",
                        "        require value == 1\n",
                        "    recycle -return 0\n",
                        "        let copy = Option.Some[Int] :: value :: call\n",
                        "        copy == 1\n",
                        "    let mut placed = Widget :: value = 0 :: call\n",
                        "    construct place Widget -> placed -return 0\n",
                        "        value = value\n",
                        "    return placed.value\n",
                    ),
                ),
                ("src/types.arc", ""),
            ],
        );
        check_path(&root).expect("same-region headed bindings should check");
        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_accepts_record_headed_regions_with_base_copy() {
        let root = make_temp_package(
            "record_headed_region_positive",
            "app",
            &[],
            &[(
                "src/shelf.arc",
                concat!(
                    "record Widget:\n",
                    "    value: Int\n",
                    "    maybe: Option[Int]\n",
                    "enum Option[T]:\n",
                    "    Some(T)\n",
                    "    None\n",
                    "fn main() -> Int:\n",
                    "    let base = construct yield Widget -return 0\n",
                    "        value = 1\n",
                    "        maybe = Option.None[Int] :: :: call\n",
                    "    let built = record yield Widget from base -return 0\n",
                    "        value = 2\n",
                    "    record deliver Widget from built -> mirrored -return 0\n",
                    "        maybe = Option.Some[Int] :: 9 :: call\n",
                    "    let mut placed = construct yield Widget -return 0\n",
                    "        value = 0\n",
                    "        maybe = Option.None[Int] :: :: call\n",
                    "    record place Widget from mirrored -> placed -return 0\n",
                    "        value = mirrored.value\n",
                    "    return placed.value\n",
                ),
            ), ("src/types.arc", "")],
        );
        check_path(&root).expect("record headed regions should check");
        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_sources_rejects_return_type_mismatches() {
        for (source, expected) in [
            (
                "fn main() -> Int:\n    return \"x\"\n",
                "return value must have type `Int`",
            ),
            (
                "fn main() -> Int:\n    return\n",
                "return statement requires a value of type `Int`",
            ),
            (
                "fn helper() -> Unit:\n    return 1\nfn main() -> Int:\n    helper :: :: call\n    return 0\n",
                "return value is not allowed because the enclosing routine returns Unit",
            ),
        ] {
            let root = make_temp_package(
                "return_type_mismatch",
                "app",
                &[],
                &[("src/shelf.arc", source), ("src/types.arc", "")],
            );
            let err = check_path(&root).expect_err("return mismatch should fail");
            assert!(err.contains(expected), "{expected}: {err}");
            fs::remove_dir_all(root).expect("cleanup should succeed");
        }
    }

    fn make_temp_package(
        name: &str,
        kind: &str,
        deps: &[(&str, &str)],
        files: &[(&str, &str)],
    ) -> PathBuf {
        let root = test_temp_dir("arcana-frontend-tests", name);
        if root.exists() {
            fs::remove_dir_all(&root).expect("stale temp dir should be removable");
        }

        fs::create_dir_all(root.join("src")).expect("src dir should be creatable");
        let mut manifest = format!("name = \"{name}\"\nkind = \"{kind}\"\n");
        if !deps.is_empty() {
            manifest.push_str("\n[deps]\n");
            for (dep_name, dep_path) in deps {
                manifest.push_str(&format!("{dep_name} = {{ path = \"{dep_path}\" }}\n"));
            }
        }
        fs::write(root.join("book.toml"), manifest).expect("manifest should be writable");

        for (rel_path, contents) in files {
            let path = root.join(rel_path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("parent dirs should be creatable");
            }
            fs::write(path, contents).expect("source file should be writable");
        }

        root
    }

    fn make_temp_workspace(name: &str, members: &[&str], files: &[(&str, &str)]) -> PathBuf {
        let root = test_temp_dir("arcana-frontend-workspaces", name);
        if root.exists() {
            fs::remove_dir_all(&root).expect("stale temp dir should be removable");
        }
        fs::create_dir_all(&root).expect("workspace dir should be creatable");
        let workspace_members = members
            .iter()
            .map(|member| format!("\"{member}\""))
            .collect::<Vec<_>>()
            .join(", ");
        fs::write(
            root.join("book.toml"),
            format!(
                "name = \"workspace\"\nkind = \"app\"\n[workspace]\nmembers = [{workspace_members}]\n"
            ),
        )
        .expect("workspace manifest should be writable");
        for (rel_path, contents) in files {
            let path = root.join(rel_path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("parent dirs should be creatable");
            }
            fs::write(path, contents).expect("file should be writable");
        }
        root
    }

    fn write_foreword_script_contents(root: &Path, name: &str, contents: &str) -> String {
        let relative = if cfg!(windows) {
            format!("forewords/{name}.cmd")
        } else {
            format!("forewords/{name}.sh")
        };
        let target = root.join(&relative);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).expect("adapter parent should be creatable");
        }
        fs::write(&target, contents).expect("adapter script should be writable");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&target)
                .expect("adapter metadata should load")
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&target, perms).expect("adapter perms should update");
        }
        relative
    }

    fn write_foreword_adapter_script(root: &Path, name: &str, output: &str) -> String {
        if cfg!(windows) {
            let relative = format!("forewords/{name}.cmd");
            let target = root.join(&relative);
            let payload_target = root.join("forewords").join(format!("{name}.json"));
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).expect("adapter parent should be creatable");
            }
            fs::write(&payload_target, output).expect("adapter payload should be writable");
            fs::write(
                &target,
                format!("@echo off\r\ntype \"%~dp0{name}.json\"\r\n"),
            )
            .expect("adapter script should be writable");
            relative
        } else {
            let script = format!("#!/bin/sh\nprintf '%s' '{output}'\n");
            write_foreword_script_contents(root, name, &script)
        }
    }

    fn make_adapter_request_fixture(
        root: &Path,
        relative_product_path: &str,
    ) -> (
        arcana_hir::HirWorkspacePackage,
        arcana_hir::HirForewordAdapterProduct,
        ForewordAdapterRequest,
    ) {
        let package = arcana_hir::HirWorkspacePackage {
            package_id: "tool.pkg".to_string(),
            root_dir: root.to_path_buf(),
            direct_deps: BTreeSet::new(),
            direct_dep_packages: BTreeMap::new(),
            direct_dep_ids: BTreeMap::new(),
            executable_foreword_deps: BTreeSet::new(),
            foreword_products: BTreeMap::new(),
            summary: arcana_hir::HirPackageSummary {
                package_name: "tool".to_string(),
                modules: Vec::new(),
                dependency_edges: Vec::new(),
            },
            layout: arcana_hir::HirPackageLayout {
                module_paths: BTreeMap::new(),
                relative_modules: BTreeMap::new(),
                absolute_modules: BTreeMap::new(),
            },
        };
        let product = arcana_hir::HirForewordAdapterProduct {
            name: "tool-forewords".to_string(),
            path: relative_product_path.to_string(),
            runner: None,
            args: Vec::new(),
        };
        let request = ForewordAdapterRequest {
            version: FOREWORD_ADAPTER_PROTOCOL_VERSION.to_string(),
            protocol: "stdio-v1".to_string(),
            cache_key: "fixture".to_string(),
            toolchain_version: "fixture".to_string(),
            dependency_opt_in_enabled: true,
            package: AdapterPackageSnapshot {
                package_id: "app.pkg".to_string(),
                package_name: "app".to_string(),
                root_dir: root.display().to_string(),
                module_id: "app".to_string(),
            },
            foreword: AdapterForewordSnapshot {
                applied_name: "tool.exec.fixture".to_string(),
                resolved_name: "tool.exec.fixture".to_string(),
                tier: "executable".to_string(),
                visibility: "public".to_string(),
                phase: "frontend".to_string(),
                action: "metadata".to_string(),
                retention: "compile".to_string(),
                targets: vec!["fn".to_string()],
                diagnostic_namespace: Some("tool.exec.fixture".to_string()),
                payload_schema: Vec::new(),
                repeatable: false,
                conflicts: Vec::new(),
                args: Vec::new(),
                provider_package_id: "tool.pkg".to_string(),
                exposed_package_id: "tool.pkg".to_string(),
                handler: Some("tool.exec.fixture_handler".to_string()),
                entry: Some("run".to_string()),
            },
            target: AdapterTargetSnapshot {
                kind: "fn".to_string(),
                path: "app.main".to_string(),
                public: true,
                owner_kind: "symbol".to_string(),
                owner_symbol: None,
                owner_directive: None,
                selected_field: None,
                selected_param: None,
                selected_method_name: None,
                container_kind: None,
                container_name: None,
            },
            visible_forewords: Vec::new(),
            artifact: AdapterArtifactIdentity {
                product_name: "tool-forewords".to_string(),
                product_path: relative_product_path.to_string(),
                runner: None,
                args: Vec::new(),
                product_digest: None,
                runner_digest: None,
            },
        };
        (package, product, request)
    }

    fn write_foreword_runner_script(
        root: &Path,
        name: &str,
        expected_flag: &str,
        output: &str,
    ) -> String {
        let relative = if cfg!(windows) {
            format!("forewords/{name}.ps1")
        } else {
            format!("forewords/{name}.sh")
        };
        let target = root.join(&relative);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).expect("runner parent should be creatable");
        }
        let script = if cfg!(windows) {
            format!(
                "param([string]$flag, [string]$productPath)\r\nif ($flag -ne '{expected_flag}') {{ exit 9 }}\r\nWrite-Output '{output}'\r\n"
            )
        } else {
            format!(
                "#!/bin/sh\nif [ \"$1\" != \"{expected_flag}\" ]; then\n  exit 9\nfi\nprintf '%s' '{output}'\n"
            )
        };
        fs::write(&target, script).expect("runner script should be writable");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&target)
                .expect("runner metadata should load")
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&target, perms).expect("runner perms should update");
        }
        relative
    }

    fn copy_test_adapter_script(source: &Path, target: &Path) {
        let bytes = fs::read(source).expect("adapter source should be readable");
        fs::write(target, bytes).expect("adapter target should be writable");
        if let Some(stem) = source.file_stem().and_then(|stem| stem.to_str()) {
            let payload_source = source.with_file_name(format!("{stem}.json"));
            if payload_source.is_file() {
                let payload_target = target.with_file_name(format!("{stem}.json"));
                fs::write(
                    &payload_target,
                    fs::read(&payload_source).expect("adapter payload should be readable"),
                )
                .expect("adapter payload should be writable");
            }
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(target)
                .expect("adapter target metadata should load")
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(target, perms).expect("adapter target perms should update");
        }
    }

    fn test_temp_dir(prefix: &str, name: &str) -> PathBuf {
        // Keep frontend fixture workspaces outside the repo tree so implicit std lookup
        // does not capture the real repository std package during package discovery.
        let root = repo_root()
            .parent()
            .expect("repo root parent should exist")
            .join("target")
            .join(prefix)
            .join(format!("{}-{}", unique_test_id(), name));
        fs::create_dir_all(root.parent().expect("temp dir parent should exist"))
            .expect("temp dir parent should be creatable");
        root
    }

    fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .canonicalize()
            .expect("repo root should resolve")
    }

    fn owned_root() -> PathBuf {
        repo_root().join("grimoires").join("owned")
    }

    fn owned_app_root() -> PathBuf {
        let libs = owned_root().join("libs");
        if libs.is_dir() {
            libs
        } else {
            owned_root().join("app")
        }
    }

    fn unique_test_id() -> u64 {
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos() as u64;
        time ^ NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed)
    }
}
