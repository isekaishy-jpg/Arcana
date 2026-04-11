#![allow(clippy::too_many_arguments)]

mod entrypoint;
mod executable;
mod routine_signature;
mod runtime_requirements;

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use serde::{Deserialize, Serialize};

use arcana_cabi::{
    ArcanaCabiBindingLayout, ArcanaCabiBindingLayoutField, ArcanaCabiBindingLayoutKind,
    ArcanaCabiBindingRawType, ArcanaCabiBindingScalarType,
};
use arcana_hir::{
    HirAssignOp, HirAssignTarget, HirBinaryOp, HirChainConnector, HirChainIntroducer, HirChainStep,
    HirCleanupFooter, HirDirectiveKind, HirExpr, HirForewordApp, HirForewordArg,
    HirHeaderAttachment, HirImplDecl, HirLocalTypeLookup, HirMatchPattern, HirModule,
    HirModuleDependency, HirModuleSummary, HirPackageSummary, HirPath, HirPhraseArg, HirPredicate,
    HirResolvedModule, HirResolvedWorkspace, HirStatement, HirStatementKind, HirSymbol,
    HirSymbolBody, HirSymbolKind, HirType, HirTypeBindingScope, HirTypeKind, HirTypeSubstitutions,
    HirUnaryOp, HirWhereClause, HirWorkspacePackage, HirWorkspaceSummary,
    canonicalize_hir_type_in_module, current_workspace_package_for_module, hir_type_matches,
    impl_target_is_public_from_package, infer_receiver_expr_type,
    lookup_method_candidates_for_hir_type, lookup_shackle_decl_path, lookup_symbol_path,
    match_name_resolves_to_zero_payload_variant, render_symbol_signature,
    routine_key_for_impl_method, routine_key_for_object_method, routine_key_for_symbol,
};
pub use entrypoint::{
    RUNTIME_MAIN_ENTRYPOINT_NAME, is_runtime_main_entry_symbol,
    validate_runtime_main_entry_contract, validate_runtime_main_entry_symbol,
};
pub use runtime_requirements::{
    RuntimeRequirementRoots, derive_runtime_requirements, derive_runtime_requirements_with_roots,
};

pub use executable::{
    ExecArrayLine, ExecArrayRegion, ExecAssignOp, ExecAssignTarget, ExecAvailabilityAttachment,
    ExecAvailabilityKind, ExecBinaryOp, ExecBindLine, ExecBindLineKind, ExecChainConnector,
    ExecChainIntroducer, ExecChainStep, ExecCleanupFooter, ExecConstructContributionMode,
    ExecConstructDestination, ExecConstructLine, ExecConstructRegion, ExecDeferAction,
    ExecDynamicDispatch, ExecExpr, ExecFloatKind, ExecHeadedModifier, ExecHeaderAttachment,
    ExecMatchArm, ExecMatchPattern, ExecMemoryDetailLine, ExecMemorySpecDecl, ExecNamedBindingId,
    ExecPhraseArg, ExecPhraseQualifierKind, ExecRecordRegion, ExecRecycleLine, ExecRecycleLineKind,
    ExecStmt, ExecUnaryOp,
};
pub use routine_signature::{
    IrRoutineParam, IrRoutineProvenance, IrRoutineType, IrRoutineTypeKind, parse_routine_type_text,
    render_routine_signature_text,
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IrModule {
    pub symbol_count: usize,
    pub item_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrPackageModule {
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrEntrypoint {
    pub package_id: String,
    pub module_id: String,
    pub symbol_name: String,
    pub symbol_kind: String,
    pub is_async: bool,
    pub exported: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum IrForewordArgValue {
    #[serde(rename = "raw")]
    Raw(String),
    #[serde(rename = "bool")]
    Bool(bool),
    #[serde(rename = "int")]
    Int(i64),
    #[serde(rename = "str")]
    Str(String),
    #[serde(rename = "symbol")]
    Symbol(String),
    #[serde(rename = "path")]
    Path(Vec<String>),
}

impl Default for IrForewordArgValue {
    fn default() -> Self {
        Self::Raw(String::new())
    }
}

impl IrForewordArgValue {
    pub fn render(&self) -> String {
        match self {
            Self::Raw(value) => value.clone(),
            Self::Bool(value) => value.to_string(),
            Self::Int(value) => value.to_string(),
            Self::Str(value) => format!("\"{}\"", quote_text(value)),
            Self::Symbol(value) => value.clone(),
            Self::Path(segments) => segments.join("."),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IrForewordArg {
    pub name: Option<String>,
    pub value: String,
    #[serde(default)]
    pub typed_value: IrForewordArgValue,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum IrForewordRetention {
    Compile,
    Tooling,
    Runtime,
}

impl IrForewordRetention {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Compile => "compile",
            Self::Tooling => "tooling",
            Self::Runtime => "runtime",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum IrForewordEntryKind {
    #[default]
    Attached,
    Generated,
    Emitted,
}

impl IrForewordEntryKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Attached => "attached",
            Self::Generated => "generated",
            Self::Emitted => "emitted",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IrForewordGeneratedBy {
    pub applied_name: String,
    pub resolved_name: String,
    pub provider_package_id: String,
    pub owner_kind: String,
    pub owner_path: String,
    pub retention: IrForewordRetention,
    pub args: Vec<IrForewordArg>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IrForewordMetadata {
    #[serde(default)]
    pub entry_kind: IrForewordEntryKind,
    pub qualified_name: String,
    pub package_id: String,
    pub module_id: String,
    pub target_kind: String,
    pub target_path: String,
    pub retention: IrForewordRetention,
    pub args: Vec<IrForewordArg>,
    pub public: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generated_by: Option<IrForewordGeneratedBy>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IrForewordRegistrationRow {
    pub namespace: String,
    pub key: String,
    pub value: String,
    pub target_kind: String,
    pub target_path: String,
    pub public: bool,
    pub generated_by: IrForewordGeneratedBy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrRoutine {
    pub package_id: String,
    pub module_id: String,
    pub routine_key: String,
    pub symbol_name: String,
    pub symbol_kind: String,
    pub exported: bool,
    pub is_async: bool,
    pub type_params: Vec<String>,
    pub behavior_attrs: BTreeMap<String, String>,
    pub params: Vec<IrRoutineParam>,
    pub return_type: Option<IrRoutineType>,
    pub intrinsic_impl: Option<String>,
    pub native_impl: Option<String>,
    pub impl_target_type: Option<IrRoutineType>,
    pub impl_trait_path: Option<Vec<String>>,
    pub availability: Vec<ExecAvailabilityAttachment>,
    pub inline_hint: bool,
    pub cold_hint: bool,
    pub cleanup_footers: Vec<ExecCleanupFooter>,
    pub statements: Vec<ExecStmt>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IrNativeCallbackDecl {
    pub package_id: String,
    pub module_id: String,
    pub name: String,
    pub params: Vec<IrRoutineParam>,
    pub return_type: Option<IrRoutineType>,
    pub callback_type: Option<IrRoutineType>,
    pub target: Vec<String>,
    pub target_routine_key: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IrShackleDecl {
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
    pub body_entries: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_layout: Option<ArcanaCabiBindingLayout>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub import_target: Option<IrShackleImportTarget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thunk_target: Option<IrShackleThunkTarget>,
    pub surface_text: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IrShackleImportTarget {
    pub library: String,
    pub symbol: String,
    pub abi: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IrShackleThunkTarget {
    pub target: String,
    pub abi: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecStructBitfieldFieldLayout {
    pub name: String,
    pub base_type: String,
    pub storage_index: u16,
    pub bit_offset: u16,
    pub bit_width: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecStructBitfieldLayout {
    pub type_name: String,
    pub fields: Vec<ExecStructBitfieldFieldLayout>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrOwnerObject {
    pub type_path: Vec<String>,
    pub local_name: String,
    pub init_routine_key: Option<String>,
    pub init_with_context_routine_key: Option<String>,
    pub resume_routine_key: Option<String>,
    pub resume_with_context_routine_key: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrOwnerExit {
    pub name: String,
    pub condition: ExecExpr,
    pub retains: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrOwnerDecl {
    pub package_id: String,
    pub module_id: String,
    pub owner_path: Vec<String>,
    pub owner_name: String,
    pub context_type: Option<IrRoutineType>,
    pub objects: Vec<IrOwnerObject>,
    pub exits: Vec<IrOwnerExit>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrPackage {
    pub package_id: String,
    pub package_name: String,
    pub root_module_id: String,
    pub direct_deps: Vec<String>,
    pub direct_dep_ids: Vec<String>,
    pub package_display_names: BTreeMap<String, String>,
    pub package_direct_dep_ids: BTreeMap<String, BTreeMap<String, String>>,
    pub modules: Vec<IrPackageModule>,
    pub dependency_edge_count: usize,
    pub dependency_rows: Vec<String>,
    pub exported_surface_rows: Vec<String>,
    pub runtime_requirements: Vec<String>,
    pub foreword_index: Vec<IrForewordMetadata>,
    pub foreword_registrations: Vec<IrForewordRegistrationRow>,
    pub entrypoints: Vec<IrEntrypoint>,
    pub routines: Vec<IrRoutine>,
    pub native_callbacks: Vec<IrNativeCallbackDecl>,
    pub shackle_decls: Vec<IrShackleDecl>,
    pub owners: Vec<IrOwnerDecl>,
}

pub fn disambiguate_package_routine_keys(package: &mut IrPackage) -> Result<(), String> {
    let mut key_packages = BTreeMap::<String, BTreeSet<String>>::new();
    for routine in &package.routines {
        key_packages
            .entry(routine.routine_key.clone())
            .or_default()
            .insert(routine.package_id.clone());
    }
    let duplicate_keys = key_packages
        .into_iter()
        .filter_map(|(routine_key, package_ids)| (package_ids.len() > 1).then_some(routine_key))
        .collect::<BTreeSet<_>>();
    if duplicate_keys.is_empty() {
        return Ok(());
    }

    let package_display_names = package.package_display_names.clone();
    let package_direct_dep_ids = package.package_direct_dep_ids.clone();
    let mut routine_key_map = BTreeMap::<(String, String), String>::new();
    for routine in &package.routines {
        let new_key = if duplicate_keys.contains(&routine.routine_key) {
            format!("{}|{}", routine.package_id, routine.routine_key)
        } else {
            routine.routine_key.clone()
        };
        routine_key_map.insert(
            (routine.package_id.clone(), routine.routine_key.clone()),
            new_key,
        );
    }

    for routine in &mut package.routines {
        let old_key = routine.routine_key.clone();
        routine.routine_key = routine_key_map
            .get(&(routine.package_id.clone(), old_key))
            .expect("routine key mapping should exist")
            .clone();
        rewrite_stmt_block_routine_keys(
            &package_display_names,
            &package_direct_dep_ids,
            &duplicate_keys,
            &routine_key_map,
            &routine.package_id,
            &mut routine.statements,
        )?;
    }

    for owner in &mut package.owners {
        for object in &mut owner.objects {
            rewrite_owner_routine_key(
                &package_display_names,
                &package_direct_dep_ids,
                &duplicate_keys,
                &routine_key_map,
                &owner.package_id,
                &mut object.init_routine_key,
            )?;
            rewrite_owner_routine_key(
                &package_display_names,
                &package_direct_dep_ids,
                &duplicate_keys,
                &routine_key_map,
                &owner.package_id,
                &mut object.init_with_context_routine_key,
            )?;
            rewrite_owner_routine_key(
                &package_display_names,
                &package_direct_dep_ids,
                &duplicate_keys,
                &routine_key_map,
                &owner.package_id,
                &mut object.resume_routine_key,
            )?;
            rewrite_owner_routine_key(
                &package_display_names,
                &package_direct_dep_ids,
                &duplicate_keys,
                &routine_key_map,
                &owner.package_id,
                &mut object.resume_with_context_routine_key,
            )?;
        }
        for owner_exit in &mut owner.exits {
            rewrite_expr_routine_keys(
                &package_display_names,
                &package_direct_dep_ids,
                &duplicate_keys,
                &routine_key_map,
                &owner.package_id,
                &mut owner_exit.condition,
            )?;
        }
    }
    for callback in &mut package.native_callbacks {
        if let Some(target_routine_key) = &mut callback.target_routine_key
            && duplicate_keys.contains(target_routine_key)
        {
            let target_package_id = resolve_duplicate_routine_target_package_id(
                &package_display_names,
                &package_direct_dep_ids,
                &callback.package_id,
                target_routine_key,
            )?;
            *target_routine_key = routine_key_map
                .get(&(target_package_id, target_routine_key.clone()))
                .ok_or_else(|| {
                    format!(
                        "missing disambiguated callback target routine key for `{}` in package `{}`",
                        callback.name, callback.package_id
                    )
                })?
                .clone();
        }
    }

    Ok(())
}

fn rewrite_owner_routine_key(
    package_display_names: &BTreeMap<String, String>,
    package_direct_dep_ids: &BTreeMap<String, BTreeMap<String, String>>,
    duplicate_keys: &BTreeSet<String>,
    routine_key_map: &BTreeMap<(String, String), String>,
    current_package_id: &str,
    routine_key: &mut Option<String>,
) -> Result<(), String> {
    let Some(existing) = routine_key.clone() else {
        return Ok(());
    };
    if !duplicate_keys.contains(&existing) {
        return Ok(());
    }
    let target_package_id = resolve_duplicate_routine_target_package_id(
        package_display_names,
        package_direct_dep_ids,
        current_package_id,
        &existing,
    )?;
    *routine_key = Some(
        routine_key_map
            .get(&(target_package_id, existing.clone()))
            .ok_or_else(|| {
                format!(
                    "missing rewritten routine key mapping for `{existing}` from package `{current_package_id}`"
                )
            })?
            .clone(),
    );
    Ok(())
}

fn rewrite_stmt_block_routine_keys(
    package_display_names: &BTreeMap<String, String>,
    package_direct_dep_ids: &BTreeMap<String, BTreeMap<String, String>>,
    duplicate_keys: &BTreeSet<String>,
    routine_key_map: &BTreeMap<(String, String), String>,
    current_package_id: &str,
    statements: &mut [ExecStmt],
) -> Result<(), String> {
    for statement in statements {
        rewrite_stmt_routine_keys(
            package_display_names,
            package_direct_dep_ids,
            duplicate_keys,
            routine_key_map,
            current_package_id,
            statement,
        )?;
    }
    Ok(())
}

fn rewrite_stmt_routine_keys(
    package_display_names: &BTreeMap<String, String>,
    package_direct_dep_ids: &BTreeMap<String, BTreeMap<String, String>>,
    duplicate_keys: &BTreeSet<String>,
    routine_key_map: &BTreeMap<(String, String), String>,
    current_package_id: &str,
    statement: &mut ExecStmt,
) -> Result<(), String> {
    match statement {
        ExecStmt::Let { value, .. } => rewrite_expr_routine_keys(
            package_display_names,
            package_direct_dep_ids,
            duplicate_keys,
            routine_key_map,
            current_package_id,
            value,
        ),
        ExecStmt::Expr { expr, .. } => rewrite_expr_routine_keys(
            package_display_names,
            package_direct_dep_ids,
            duplicate_keys,
            routine_key_map,
            current_package_id,
            expr,
        ),
        ExecStmt::Reclaim(expr) => rewrite_expr_routine_keys(
            package_display_names,
            package_direct_dep_ids,
            duplicate_keys,
            routine_key_map,
            current_package_id,
            expr,
        ),
        ExecStmt::ReturnVoid | ExecStmt::Break | ExecStmt::Continue => Ok(()),
        ExecStmt::ReturnValue { value } => rewrite_expr_routine_keys(
            package_display_names,
            package_direct_dep_ids,
            duplicate_keys,
            routine_key_map,
            current_package_id,
            value,
        ),
        ExecStmt::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            rewrite_expr_routine_keys(
                package_display_names,
                package_direct_dep_ids,
                duplicate_keys,
                routine_key_map,
                current_package_id,
                condition,
            )?;
            rewrite_stmt_block_routine_keys(
                package_display_names,
                package_direct_dep_ids,
                duplicate_keys,
                routine_key_map,
                current_package_id,
                then_branch,
            )?;
            rewrite_stmt_block_routine_keys(
                package_display_names,
                package_direct_dep_ids,
                duplicate_keys,
                routine_key_map,
                current_package_id,
                else_branch,
            )
        }
        ExecStmt::While {
            condition, body, ..
        } => {
            rewrite_expr_routine_keys(
                package_display_names,
                package_direct_dep_ids,
                duplicate_keys,
                routine_key_map,
                current_package_id,
                condition,
            )?;
            rewrite_stmt_block_routine_keys(
                package_display_names,
                package_direct_dep_ids,
                duplicate_keys,
                routine_key_map,
                current_package_id,
                body,
            )
        }
        ExecStmt::For { iterable, body, .. } => {
            rewrite_expr_routine_keys(
                package_display_names,
                package_direct_dep_ids,
                duplicate_keys,
                routine_key_map,
                current_package_id,
                iterable,
            )?;
            rewrite_stmt_block_routine_keys(
                package_display_names,
                package_direct_dep_ids,
                duplicate_keys,
                routine_key_map,
                current_package_id,
                body,
            )
        }
        ExecStmt::ActivateOwner { context, .. } => {
            if let Some(context) = context {
                rewrite_expr_routine_keys(
                    package_display_names,
                    package_direct_dep_ids,
                    duplicate_keys,
                    routine_key_map,
                    current_package_id,
                    context,
                )?;
            }
            Ok(())
        }
        ExecStmt::Defer(action) => match action {
            ExecDeferAction::Expr(expr) | ExecDeferAction::Reclaim(expr) => {
                rewrite_expr_routine_keys(
                    package_display_names,
                    package_direct_dep_ids,
                    duplicate_keys,
                    routine_key_map,
                    current_package_id,
                    expr,
                )
            }
        },
        ExecStmt::Assign { target, value, .. } => {
            rewrite_assign_target_routine_keys(
                package_display_names,
                package_direct_dep_ids,
                duplicate_keys,
                routine_key_map,
                current_package_id,
                target,
            )?;
            rewrite_expr_routine_keys(
                package_display_names,
                package_direct_dep_ids,
                duplicate_keys,
                routine_key_map,
                current_package_id,
                value,
            )
        }
        ExecStmt::Recycle {
            default_modifier,
            lines,
        } => {
            if let Some(modifier) = default_modifier
                && let Some(payload) = &mut modifier.payload
            {
                rewrite_expr_routine_keys(
                    package_display_names,
                    package_direct_dep_ids,
                    duplicate_keys,
                    routine_key_map,
                    current_package_id,
                    payload,
                )?;
            }
            for line in lines {
                match &mut line.kind {
                    ExecRecycleLineKind::Expr { gate }
                    | ExecRecycleLineKind::Let { gate, .. }
                    | ExecRecycleLineKind::Assign { gate, .. } => rewrite_expr_routine_keys(
                        package_display_names,
                        package_direct_dep_ids,
                        duplicate_keys,
                        routine_key_map,
                        current_package_id,
                        gate,
                    )?,
                }
                if let Some(modifier) = &mut line.modifier
                    && let Some(payload) = &mut modifier.payload
                {
                    rewrite_expr_routine_keys(
                        package_display_names,
                        package_direct_dep_ids,
                        duplicate_keys,
                        routine_key_map,
                        current_package_id,
                        payload,
                    )?;
                }
            }
            Ok(())
        }
        ExecStmt::Bind {
            default_modifier,
            lines,
        } => {
            if let Some(modifier) = default_modifier
                && let Some(payload) = &mut modifier.payload
            {
                rewrite_expr_routine_keys(
                    package_display_names,
                    package_direct_dep_ids,
                    duplicate_keys,
                    routine_key_map,
                    current_package_id,
                    payload,
                )?;
            }
            for line in lines {
                match &mut line.kind {
                    ExecBindLineKind::Let { gate, .. } | ExecBindLineKind::Assign { gate, .. } => {
                        rewrite_expr_routine_keys(
                            package_display_names,
                            package_direct_dep_ids,
                            duplicate_keys,
                            routine_key_map,
                            current_package_id,
                            gate,
                        )?
                    }
                    ExecBindLineKind::Require { expr } => rewrite_expr_routine_keys(
                        package_display_names,
                        package_direct_dep_ids,
                        duplicate_keys,
                        routine_key_map,
                        current_package_id,
                        expr,
                    )?,
                }
                if let Some(modifier) = &mut line.modifier
                    && let Some(payload) = &mut modifier.payload
                {
                    rewrite_expr_routine_keys(
                        package_display_names,
                        package_direct_dep_ids,
                        duplicate_keys,
                        routine_key_map,
                        current_package_id,
                        payload,
                    )?;
                }
            }
            Ok(())
        }
        ExecStmt::Construct(region) => {
            rewrite_expr_routine_keys(
                package_display_names,
                package_direct_dep_ids,
                duplicate_keys,
                routine_key_map,
                current_package_id,
                &mut region.target,
            )?;
            if let Some(ExecConstructDestination::Place { target }) = &mut region.destination {
                rewrite_assign_target_routine_keys(
                    package_display_names,
                    package_direct_dep_ids,
                    duplicate_keys,
                    routine_key_map,
                    current_package_id,
                    target,
                )?;
            }
            if let Some(modifier) = &mut region.default_modifier
                && let Some(payload) = &mut modifier.payload
            {
                rewrite_expr_routine_keys(
                    package_display_names,
                    package_direct_dep_ids,
                    duplicate_keys,
                    routine_key_map,
                    current_package_id,
                    payload,
                )?;
            }
            for line in &mut region.lines {
                rewrite_expr_routine_keys(
                    package_display_names,
                    package_direct_dep_ids,
                    duplicate_keys,
                    routine_key_map,
                    current_package_id,
                    &mut line.value,
                )?;
                if let Some(modifier) = &mut line.modifier
                    && let Some(payload) = &mut modifier.payload
                {
                    rewrite_expr_routine_keys(
                        package_display_names,
                        package_direct_dep_ids,
                        duplicate_keys,
                        routine_key_map,
                        current_package_id,
                        payload,
                    )?;
                }
            }
            Ok(())
        }
        ExecStmt::Record(region) => {
            rewrite_expr_routine_keys(
                package_display_names,
                package_direct_dep_ids,
                duplicate_keys,
                routine_key_map,
                current_package_id,
                &mut region.target,
            )?;
            if let Some(base) = &mut region.base {
                rewrite_expr_routine_keys(
                    package_display_names,
                    package_direct_dep_ids,
                    duplicate_keys,
                    routine_key_map,
                    current_package_id,
                    base,
                )?;
            }
            if let Some(ExecConstructDestination::Place { target }) = &mut region.destination {
                rewrite_assign_target_routine_keys(
                    package_display_names,
                    package_direct_dep_ids,
                    duplicate_keys,
                    routine_key_map,
                    current_package_id,
                    target,
                )?;
            }
            if let Some(modifier) = &mut region.default_modifier
                && let Some(payload) = &mut modifier.payload
            {
                rewrite_expr_routine_keys(
                    package_display_names,
                    package_direct_dep_ids,
                    duplicate_keys,
                    routine_key_map,
                    current_package_id,
                    payload,
                )?;
            }
            for line in &mut region.lines {
                rewrite_expr_routine_keys(
                    package_display_names,
                    package_direct_dep_ids,
                    duplicate_keys,
                    routine_key_map,
                    current_package_id,
                    &mut line.value,
                )?;
                if let Some(modifier) = &mut line.modifier
                    && let Some(payload) = &mut modifier.payload
                {
                    rewrite_expr_routine_keys(
                        package_display_names,
                        package_direct_dep_ids,
                        duplicate_keys,
                        routine_key_map,
                        current_package_id,
                        payload,
                    )?;
                }
            }
            Ok(())
        }
        ExecStmt::Array(region) => {
            rewrite_expr_routine_keys(
                package_display_names,
                package_direct_dep_ids,
                duplicate_keys,
                routine_key_map,
                current_package_id,
                &mut region.target,
            )?;
            if let Some(base) = &mut region.base {
                rewrite_expr_routine_keys(
                    package_display_names,
                    package_direct_dep_ids,
                    duplicate_keys,
                    routine_key_map,
                    current_package_id,
                    base,
                )?;
            }
            if let Some(ExecConstructDestination::Place { target }) = &mut region.destination {
                rewrite_assign_target_routine_keys(
                    package_display_names,
                    package_direct_dep_ids,
                    duplicate_keys,
                    routine_key_map,
                    current_package_id,
                    target,
                )?;
            }
            if let Some(modifier) = &mut region.default_modifier
                && let Some(payload) = &mut modifier.payload
            {
                rewrite_expr_routine_keys(
                    package_display_names,
                    package_direct_dep_ids,
                    duplicate_keys,
                    routine_key_map,
                    current_package_id,
                    payload,
                )?;
            }
            for line in &mut region.lines {
                rewrite_expr_routine_keys(
                    package_display_names,
                    package_direct_dep_ids,
                    duplicate_keys,
                    routine_key_map,
                    current_package_id,
                    &mut line.value,
                )?;
                if let Some(modifier) = &mut line.modifier
                    && let Some(payload) = &mut modifier.payload
                {
                    rewrite_expr_routine_keys(
                        package_display_names,
                        package_direct_dep_ids,
                        duplicate_keys,
                        routine_key_map,
                        current_package_id,
                        payload,
                    )?;
                }
            }
            Ok(())
        }
        ExecStmt::MemorySpec(spec) => {
            if let Some(modifier) = &mut spec.default_modifier
                && let Some(payload) = &mut modifier.payload
            {
                rewrite_expr_routine_keys(
                    package_display_names,
                    package_direct_dep_ids,
                    duplicate_keys,
                    routine_key_map,
                    current_package_id,
                    payload,
                )?;
            }
            for detail in &mut spec.details {
                rewrite_expr_routine_keys(
                    package_display_names,
                    package_direct_dep_ids,
                    duplicate_keys,
                    routine_key_map,
                    current_package_id,
                    &mut detail.value,
                )?;
                if let Some(modifier) = &mut detail.modifier
                    && let Some(payload) = &mut modifier.payload
                {
                    rewrite_expr_routine_keys(
                        package_display_names,
                        package_direct_dep_ids,
                        duplicate_keys,
                        routine_key_map,
                        current_package_id,
                        payload,
                    )?;
                }
            }
            Ok(())
        }
    }
}

fn rewrite_assign_target_routine_keys(
    package_display_names: &BTreeMap<String, String>,
    package_direct_dep_ids: &BTreeMap<String, BTreeMap<String, String>>,
    duplicate_keys: &BTreeSet<String>,
    routine_key_map: &BTreeMap<(String, String), String>,
    current_package_id: &str,
    target: &mut ExecAssignTarget,
) -> Result<(), String> {
    match target {
        ExecAssignTarget::Name(_) => Ok(()),
        ExecAssignTarget::Member { target, .. } => rewrite_assign_target_routine_keys(
            package_display_names,
            package_direct_dep_ids,
            duplicate_keys,
            routine_key_map,
            current_package_id,
            target,
        ),
        ExecAssignTarget::Index { target, index } => {
            rewrite_assign_target_routine_keys(
                package_display_names,
                package_direct_dep_ids,
                duplicate_keys,
                routine_key_map,
                current_package_id,
                target,
            )?;
            rewrite_expr_routine_keys(
                package_display_names,
                package_direct_dep_ids,
                duplicate_keys,
                routine_key_map,
                current_package_id,
                index,
            )
        }
    }
}

fn rewrite_expr_routine_keys(
    package_display_names: &BTreeMap<String, String>,
    package_direct_dep_ids: &BTreeMap<String, BTreeMap<String, String>>,
    duplicate_keys: &BTreeSet<String>,
    routine_key_map: &BTreeMap<(String, String), String>,
    current_package_id: &str,
    expr: &mut ExecExpr,
) -> Result<(), String> {
    match expr {
        ExecExpr::Int(_)
        | ExecExpr::Float { .. }
        | ExecExpr::Bool(_)
        | ExecExpr::Str(_)
        | ExecExpr::Path(_) => Ok(()),
        ExecExpr::Pair { left, right } | ExecExpr::Binary { left, right, .. } => {
            rewrite_expr_routine_keys(
                package_display_names,
                package_direct_dep_ids,
                duplicate_keys,
                routine_key_map,
                current_package_id,
                left,
            )?;
            rewrite_expr_routine_keys(
                package_display_names,
                package_direct_dep_ids,
                duplicate_keys,
                routine_key_map,
                current_package_id,
                right,
            )
        }
        ExecExpr::Collection { items } => {
            for item in items {
                rewrite_expr_routine_keys(
                    package_display_names,
                    package_direct_dep_ids,
                    duplicate_keys,
                    routine_key_map,
                    current_package_id,
                    item,
                )?;
            }
            Ok(())
        }
        ExecExpr::Match { subject, arms } => {
            rewrite_expr_routine_keys(
                package_display_names,
                package_direct_dep_ids,
                duplicate_keys,
                routine_key_map,
                current_package_id,
                subject,
            )?;
            for arm in arms {
                rewrite_expr_routine_keys(
                    package_display_names,
                    package_direct_dep_ids,
                    duplicate_keys,
                    routine_key_map,
                    current_package_id,
                    &mut arm.value,
                )?;
            }
            Ok(())
        }
        ExecExpr::ConstructRegion(region) => {
            rewrite_expr_routine_keys(
                package_display_names,
                package_direct_dep_ids,
                duplicate_keys,
                routine_key_map,
                current_package_id,
                &mut region.target,
            )?;
            if let Some(ExecConstructDestination::Place { target }) = &mut region.destination {
                rewrite_assign_target_routine_keys(
                    package_display_names,
                    package_direct_dep_ids,
                    duplicate_keys,
                    routine_key_map,
                    current_package_id,
                    target,
                )?;
            }
            if let Some(modifier) = &mut region.default_modifier
                && let Some(payload) = &mut modifier.payload
            {
                rewrite_expr_routine_keys(
                    package_display_names,
                    package_direct_dep_ids,
                    duplicate_keys,
                    routine_key_map,
                    current_package_id,
                    payload,
                )?;
            }
            for line in &mut region.lines {
                rewrite_expr_routine_keys(
                    package_display_names,
                    package_direct_dep_ids,
                    duplicate_keys,
                    routine_key_map,
                    current_package_id,
                    &mut line.value,
                )?;
                if let Some(modifier) = &mut line.modifier
                    && let Some(payload) = &mut modifier.payload
                {
                    rewrite_expr_routine_keys(
                        package_display_names,
                        package_direct_dep_ids,
                        duplicate_keys,
                        routine_key_map,
                        current_package_id,
                        payload,
                    )?;
                }
            }
            Ok(())
        }
        ExecExpr::RecordRegion(region) => {
            rewrite_expr_routine_keys(
                package_display_names,
                package_direct_dep_ids,
                duplicate_keys,
                routine_key_map,
                current_package_id,
                &mut region.target,
            )?;
            if let Some(base) = &mut region.base {
                rewrite_expr_routine_keys(
                    package_display_names,
                    package_direct_dep_ids,
                    duplicate_keys,
                    routine_key_map,
                    current_package_id,
                    base,
                )?;
            }
            if let Some(ExecConstructDestination::Place { target }) = &mut region.destination {
                rewrite_assign_target_routine_keys(
                    package_display_names,
                    package_direct_dep_ids,
                    duplicate_keys,
                    routine_key_map,
                    current_package_id,
                    target,
                )?;
            }
            if let Some(modifier) = &mut region.default_modifier
                && let Some(payload) = &mut modifier.payload
            {
                rewrite_expr_routine_keys(
                    package_display_names,
                    package_direct_dep_ids,
                    duplicate_keys,
                    routine_key_map,
                    current_package_id,
                    payload,
                )?;
            }
            for line in &mut region.lines {
                rewrite_expr_routine_keys(
                    package_display_names,
                    package_direct_dep_ids,
                    duplicate_keys,
                    routine_key_map,
                    current_package_id,
                    &mut line.value,
                )?;
                if let Some(modifier) = &mut line.modifier
                    && let Some(payload) = &mut modifier.payload
                {
                    rewrite_expr_routine_keys(
                        package_display_names,
                        package_direct_dep_ids,
                        duplicate_keys,
                        routine_key_map,
                        current_package_id,
                        payload,
                    )?;
                }
            }
            Ok(())
        }
        ExecExpr::ArrayRegion(region) => {
            rewrite_expr_routine_keys(
                package_display_names,
                package_direct_dep_ids,
                duplicate_keys,
                routine_key_map,
                current_package_id,
                &mut region.target,
            )?;
            if let Some(base) = &mut region.base {
                rewrite_expr_routine_keys(
                    package_display_names,
                    package_direct_dep_ids,
                    duplicate_keys,
                    routine_key_map,
                    current_package_id,
                    base,
                )?;
            }
            if let Some(ExecConstructDestination::Place { target }) = &mut region.destination {
                rewrite_assign_target_routine_keys(
                    package_display_names,
                    package_direct_dep_ids,
                    duplicate_keys,
                    routine_key_map,
                    current_package_id,
                    target,
                )?;
            }
            if let Some(modifier) = &mut region.default_modifier
                && let Some(payload) = &mut modifier.payload
            {
                rewrite_expr_routine_keys(
                    package_display_names,
                    package_direct_dep_ids,
                    duplicate_keys,
                    routine_key_map,
                    current_package_id,
                    payload,
                )?;
            }
            for line in &mut region.lines {
                rewrite_expr_routine_keys(
                    package_display_names,
                    package_direct_dep_ids,
                    duplicate_keys,
                    routine_key_map,
                    current_package_id,
                    &mut line.value,
                )?;
                if let Some(modifier) = &mut line.modifier
                    && let Some(payload) = &mut modifier.payload
                {
                    rewrite_expr_routine_keys(
                        package_display_names,
                        package_direct_dep_ids,
                        duplicate_keys,
                        routine_key_map,
                        current_package_id,
                        payload,
                    )?;
                }
            }
            Ok(())
        }
        ExecExpr::Chain { steps, .. } => {
            for step in steps {
                rewrite_chain_step_routine_keys(
                    package_display_names,
                    package_direct_dep_ids,
                    duplicate_keys,
                    routine_key_map,
                    current_package_id,
                    step,
                )?;
            }
            Ok(())
        }
        ExecExpr::MemoryPhrase {
            arena,
            init_args,
            constructor,
            attached,
            ..
        } => {
            rewrite_expr_routine_keys(
                package_display_names,
                package_direct_dep_ids,
                duplicate_keys,
                routine_key_map,
                current_package_id,
                arena,
            )?;
            for arg in init_args {
                rewrite_expr_routine_keys(
                    package_display_names,
                    package_direct_dep_ids,
                    duplicate_keys,
                    routine_key_map,
                    current_package_id,
                    &mut arg.value,
                )?;
            }
            rewrite_expr_routine_keys(
                package_display_names,
                package_direct_dep_ids,
                duplicate_keys,
                routine_key_map,
                current_package_id,
                constructor,
            )?;
            rewrite_header_attachments_routine_keys(
                package_display_names,
                package_direct_dep_ids,
                duplicate_keys,
                routine_key_map,
                current_package_id,
                attached,
            )
        }
        ExecExpr::Member { expr, .. }
        | ExecExpr::Await { expr }
        | ExecExpr::Unary { expr, .. }
        | ExecExpr::Generic { expr, .. } => rewrite_expr_routine_keys(
            package_display_names,
            package_direct_dep_ids,
            duplicate_keys,
            routine_key_map,
            current_package_id,
            expr,
        ),
        ExecExpr::Index { expr, index } => {
            rewrite_expr_routine_keys(
                package_display_names,
                package_direct_dep_ids,
                duplicate_keys,
                routine_key_map,
                current_package_id,
                expr,
            )?;
            rewrite_expr_routine_keys(
                package_display_names,
                package_direct_dep_ids,
                duplicate_keys,
                routine_key_map,
                current_package_id,
                index,
            )
        }
        ExecExpr::Slice {
            expr, start, end, ..
        } => {
            rewrite_expr_routine_keys(
                package_display_names,
                package_direct_dep_ids,
                duplicate_keys,
                routine_key_map,
                current_package_id,
                expr,
            )?;
            if let Some(start) = start {
                rewrite_expr_routine_keys(
                    package_display_names,
                    package_direct_dep_ids,
                    duplicate_keys,
                    routine_key_map,
                    current_package_id,
                    start,
                )?;
            }
            if let Some(end) = end {
                rewrite_expr_routine_keys(
                    package_display_names,
                    package_direct_dep_ids,
                    duplicate_keys,
                    routine_key_map,
                    current_package_id,
                    end,
                )?;
            }
            Ok(())
        }
        ExecExpr::Range { start, end, .. } => {
            if let Some(start) = start {
                rewrite_expr_routine_keys(
                    package_display_names,
                    package_direct_dep_ids,
                    duplicate_keys,
                    routine_key_map,
                    current_package_id,
                    start,
                )?;
            }
            if let Some(end) = end {
                rewrite_expr_routine_keys(
                    package_display_names,
                    package_direct_dep_ids,
                    duplicate_keys,
                    routine_key_map,
                    current_package_id,
                    end,
                )?;
            }
            Ok(())
        }
        ExecExpr::Phrase {
            subject,
            args,
            resolved_routine,
            attached,
            ..
        } => {
            rewrite_expr_routine_keys(
                package_display_names,
                package_direct_dep_ids,
                duplicate_keys,
                routine_key_map,
                current_package_id,
                subject,
            )?;
            for arg in args {
                rewrite_expr_routine_keys(
                    package_display_names,
                    package_direct_dep_ids,
                    duplicate_keys,
                    routine_key_map,
                    current_package_id,
                    &mut arg.value,
                )?;
            }
            rewrite_owner_routine_key(
                package_display_names,
                package_direct_dep_ids,
                duplicate_keys,
                routine_key_map,
                current_package_id,
                resolved_routine,
            )?;
            rewrite_header_attachments_routine_keys(
                package_display_names,
                package_direct_dep_ids,
                duplicate_keys,
                routine_key_map,
                current_package_id,
                attached,
            )
        }
    }
}

fn rewrite_chain_step_routine_keys(
    package_display_names: &BTreeMap<String, String>,
    package_direct_dep_ids: &BTreeMap<String, BTreeMap<String, String>>,
    duplicate_keys: &BTreeSet<String>,
    routine_key_map: &BTreeMap<(String, String), String>,
    current_package_id: &str,
    step: &mut ExecChainStep,
) -> Result<(), String> {
    rewrite_expr_routine_keys(
        package_display_names,
        package_direct_dep_ids,
        duplicate_keys,
        routine_key_map,
        current_package_id,
        &mut step.stage,
    )?;
    for bind_arg in &mut step.bind_args {
        rewrite_expr_routine_keys(
            package_display_names,
            package_direct_dep_ids,
            duplicate_keys,
            routine_key_map,
            current_package_id,
            bind_arg,
        )?;
    }
    Ok(())
}

fn rewrite_header_attachments_routine_keys(
    package_display_names: &BTreeMap<String, String>,
    package_direct_dep_ids: &BTreeMap<String, BTreeMap<String, String>>,
    duplicate_keys: &BTreeSet<String>,
    routine_key_map: &BTreeMap<(String, String), String>,
    current_package_id: &str,
    attachments: &mut [ExecHeaderAttachment],
) -> Result<(), String> {
    for attachment in attachments {
        match attachment {
            ExecHeaderAttachment::Named { value, .. }
            | ExecHeaderAttachment::Chain { expr: value } => rewrite_expr_routine_keys(
                package_display_names,
                package_direct_dep_ids,
                duplicate_keys,
                routine_key_map,
                current_package_id,
                value,
            )?,
        }
    }
    Ok(())
}

fn resolve_duplicate_routine_target_package_id(
    package_display_names: &BTreeMap<String, String>,
    package_direct_dep_ids: &BTreeMap<String, BTreeMap<String, String>>,
    current_package_id: &str,
    routine_key: &str,
) -> Result<String, String> {
    let (module_id, _) = routine_key
        .split_once('#')
        .ok_or_else(|| format!("lowered routine key `{routine_key}` is missing `#`"))?;
    let root = module_id.split('.').next().unwrap_or(module_id);
    let current_name = package_display_names
        .get(current_package_id)
        .ok_or_else(|| format!("missing display name for package `{current_package_id}`"))?;
    if root == current_name {
        return Ok(current_package_id.to_string());
    }
    package_direct_dep_ids
        .get(current_package_id)
        .and_then(|deps| deps.get(root))
        .cloned()
        .ok_or_else(|| {
            format!(
                "unable to resolve duplicate lowered routine `{routine_key}` from package `{current_package_id}`"
            )
        })
}

impl IrPackage {
    pub fn module_count(&self) -> usize {
        self.modules.len()
    }
}

#[derive(Clone, Debug)]
struct LowerValueScope {
    locals: BTreeMap<String, LowerValueBinding>,
    owner_member_types: BTreeMap<String, BTreeMap<String, HirType>>,
    next_binding_id: Rc<Cell<u64>>,
}

#[derive(Clone, Debug)]
struct LowerValueBinding {
    binding_id: u64,
    ty: HirType,
}

impl Default for LowerValueScope {
    fn default() -> Self {
        Self {
            locals: BTreeMap::new(),
            owner_member_types: BTreeMap::new(),
            next_binding_id: Rc::new(Cell::new(1)),
        }
    }
}

impl LowerValueScope {
    fn contains(&self, name: &str) -> bool {
        self.locals.contains_key(name)
    }

    fn type_of(&self, name: &str) -> Option<&HirType> {
        self.locals.get(name).map(|binding| &binding.ty)
    }

    fn binding_id_of(&self, name: &str) -> Option<u64> {
        self.locals.get(name).map(|binding| binding.binding_id)
    }

    fn insert(&mut self, name: impl Into<String>, ty: HirType) -> u64 {
        let binding_id = self.next_binding_id.get();
        self.next_binding_id.set(binding_id + 1);
        self.locals
            .insert(name.into(), LowerValueBinding { binding_id, ty });
        binding_id
    }

    fn fresh_temp_name(&self, prefix: &str) -> String {
        format!("__arcana_{prefix}_{}", self.next_binding_id.get())
    }

    fn update_type(&mut self, name: &str, ty: HirType) {
        if let Some(binding) = self.locals.get_mut(name) {
            binding.ty = ty;
        }
    }

    fn owner_member_type(&self, owner_name: &str, member: &str) -> Option<&HirType> {
        self.owner_member_types
            .get(owner_name)
            .and_then(|members| members.get(member))
    }

    fn activate_owner(
        &mut self,
        owner_local_name: &str,
        owner_path: &[String],
        objects: &[(String, HirType)],
        explicit_binding: Option<&str>,
    ) -> Vec<(String, u64)> {
        let owner_type = synthetic_hir_type(format!("Owner<{}>", owner_path.join(".")));
        let mut owner_members = BTreeMap::new();
        let mut inserted = Vec::new();
        inserted.push((
            owner_local_name.to_string(),
            self.insert(owner_local_name.to_string(), owner_type.clone()),
        ));
        for (local_name, ty) in objects {
            inserted.push((
                local_name.clone(),
                self.insert(local_name.clone(), ty.clone()),
            ));
            owner_members.insert(local_name.clone(), ty.clone());
        }
        self.owner_member_types
            .insert(owner_local_name.to_string(), owner_members.clone());
        if let Some(binding) = explicit_binding {
            inserted.push((
                binding.to_string(),
                self.insert(binding.to_string(), owner_type),
            ));
            self.owner_member_types
                .insert(binding.to_string(), owner_members);
        }
        inserted
    }
}

impl HirLocalTypeLookup for LowerValueScope {
    fn contains_local(&self, name: &str) -> bool {
        LowerValueScope::contains(self, name)
    }

    fn type_of(&self, name: &str) -> Option<&HirType> {
        LowerValueScope::type_of(self, name)
    }
}

fn simple_hir_type(name: &str) -> HirType {
    synthetic_hir_type(name.to_string())
}

fn synthetic_hir_type(name: String) -> HirType {
    HirType {
        kind: HirTypeKind::Path(HirPath {
            segments: vec![name],
            span: Default::default(),
        }),
        span: Default::default(),
    }
}

fn pair_hir_type(left: HirType, right: HirType) -> HirType {
    HirType {
        kind: HirTypeKind::Tuple(vec![left, right]),
        span: Default::default(),
    }
}

fn hir_path_matches(path: &HirPath, expected: &[&str]) -> bool {
    path.segments
        .iter()
        .map(String::as_str)
        .eq(expected.iter().copied())
}

fn hir_path_matches_any(path: &HirPath, expected: &[&[&str]]) -> bool {
    expected
        .iter()
        .any(|candidate| hir_path_matches(path, candidate))
}

#[derive(Clone, Debug)]
struct ResolvedRenderScope<'a> {
    workspace: &'a HirWorkspaceSummary,
    resolved_module: &'a HirResolvedModule,
    current_where_clause: Option<HirWhereClause>,
    type_bindings: HirTypeBindingScope,
    value_scope: LowerValueScope,
    errors: Rc<RefCell<Vec<String>>>,
}

impl<'a> ResolvedRenderScope<'a> {
    fn new(
        workspace: &'a HirWorkspaceSummary,
        resolved_module: &'a HirResolvedModule,
        current_where_clause: Option<HirWhereClause>,
        type_params: &[String],
    ) -> Self {
        Self {
            workspace,
            resolved_module,
            current_where_clause,
            type_bindings: HirTypeBindingScope::from_names(type_params.iter().cloned()),
            value_scope: LowerValueScope::default(),
            errors: Rc::new(RefCell::new(Vec::new())),
        }
    }

    fn note_error(&self, message: impl Into<String>) {
        self.errors.borrow_mut().push(message.into());
    }

    fn finish(self) -> Result<(), String> {
        let errors = self.errors.borrow();
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("\n"))
        }
    }
}

pub fn lower_hir(module: &HirModule) -> IrModule {
    IrModule {
        symbol_count: module.symbol_count,
        item_count: module.item_count,
    }
}

pub fn lower_module_summary(module: &HirModuleSummary) -> IrModule {
    IrModule {
        symbol_count: module.symbols.len(),
        item_count: module.non_empty_line_count + module.directives.len(),
    }
}

fn render_directive_row(
    module_id: &str,
    kind: HirDirectiveKind,
    path: &[String],
    alias: &Option<String>,
) -> String {
    format!(
        "module={module_id}:{}:{}:{}",
        kind.as_str(),
        path.join("."),
        alias.as_deref().unwrap_or("")
    )
}

fn render_lang_item_row(module_id: &str, name: &str, target: &[String]) -> String {
    format!("module={module_id}:lang:{name}:{}", target.join("."))
}

const MEMORY_SPEC_SURFACE_ROW_PREFIX: &str = "memory_spec:";
const STRUCT_BITFIELD_LAYOUT_ROW_PREFIX: &str = "struct_bitfield_layout:";

pub fn render_memory_spec_surface_row(spec: &ExecMemorySpecDecl) -> Result<String, String> {
    serde_json::to_string(spec)
        .map(|json| format!("{MEMORY_SPEC_SURFACE_ROW_PREFIX}{json}"))
        .map_err(|err| format!("failed to render memory spec surface row: {err}"))
}

pub fn parse_memory_spec_surface_row(row: &str) -> Result<Option<ExecMemorySpecDecl>, String> {
    let Some(json) = row.strip_prefix(MEMORY_SPEC_SURFACE_ROW_PREFIX) else {
        return Ok(None);
    };
    serde_json::from_str(json)
        .map(Some)
        .map_err(|err| format!("failed to parse memory spec surface row: {err}"))
}

pub fn render_struct_bitfield_layout_row(
    layout: &ExecStructBitfieldLayout,
) -> Result<String, String> {
    serde_json::to_string(layout)
        .map(|json| format!("{STRUCT_BITFIELD_LAYOUT_ROW_PREFIX}{json}"))
        .map_err(|err| format!("failed to render struct bitfield layout row: {err}"))
}

pub fn parse_struct_bitfield_layout_row(
    row: &str,
) -> Result<Option<ExecStructBitfieldLayout>, String> {
    let Some(json) = row.strip_prefix(STRUCT_BITFIELD_LAYOUT_ROW_PREFIX) else {
        return Ok(None);
    };
    serde_json::from_str(json)
        .map(Some)
        .map_err(|err| format!("failed to parse struct bitfield layout row: {err}"))
}

fn fixed_width_builtin_bits_for_hir_type(ty: &HirType) -> Option<u16> {
    let root = match &ty.kind {
        HirTypeKind::Path(path) => path.segments.last()?.as_str(),
        HirTypeKind::Apply { base, .. } => base.segments.last()?.as_str(),
        HirTypeKind::Ref { inner, .. } => return fixed_width_builtin_bits_for_hir_type(inner),
        _ => return None,
    };
    match root {
        "I8" | "U8" => Some(8),
        "I16" | "U16" => Some(16),
        "I32" | "U32" => Some(32),
        "I64" | "U64" => Some(64),
        _ => None,
    }
}

fn lower_struct_bitfield_layout(
    module_id: &str,
    symbol: &HirSymbol,
    fields: &[arcana_hir::HirField],
) -> Option<ExecStructBitfieldLayout> {
    let mut lowered = Vec::new();
    let mut current_storage: Option<(String, u16, u16, u16)> = None;
    let mut next_storage_index = 0_u16;
    for field in fields {
        let Some(bit_width) = field.bit_width else {
            current_storage = None;
            continue;
        };
        let base_type = field.ty.render();
        let Some(base_width) = fixed_width_builtin_bits_for_hir_type(&field.ty) else {
            current_storage = None;
            continue;
        };
        let (storage_index, bit_offset) = match current_storage.as_mut() {
            Some((current_base, current_base_width, current_index, current_offset))
                if *current_base == base_type
                    && *current_base_width == base_width
                    && current_offset.saturating_add(bit_width) <= base_width =>
            {
                let offset = *current_offset;
                *current_offset += bit_width;
                (*current_index, offset)
            }
            _ => {
                let storage_index = next_storage_index;
                next_storage_index += 1;
                current_storage = Some((base_type.clone(), base_width, storage_index, bit_width));
                (storage_index, 0)
            }
        };
        lowered.push(ExecStructBitfieldFieldLayout {
            name: field.name.clone(),
            base_type,
            storage_index,
            bit_offset,
            bit_width,
        });
    }
    (!lowered.is_empty()).then(|| ExecStructBitfieldLayout {
        type_name: format!("{module_id}.{}", symbol.name),
        fields: lowered,
    })
}

fn module_struct_bitfield_layout_rows(module: &HirModuleSummary) -> Vec<String> {
    module
        .symbols
        .iter()
        .filter_map(|symbol| match &symbol.body {
            HirSymbolBody::Struct { fields } => {
                lower_struct_bitfield_layout(&module.module_id, symbol, fields)
            }
            _ => None,
        })
        .filter_map(|layout| render_struct_bitfield_layout_row(&layout).ok())
        .collect()
}

fn resolved_module_lang_item_rows(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module: &HirModuleSummary,
) -> Vec<String> {
    let mut rows = module
        .lang_items
        .iter()
        .map(|item| {
            let target = lookup_symbol_path(workspace, resolved_module, &item.target)
                .map(resolved_symbol_path)
                .unwrap_or_else(|| item.target.clone());
            render_lang_item_row(&module.module_id, &item.name, &target)
        })
        .collect::<Vec<_>>();
    rows.extend(module_struct_bitfield_layout_rows(module));
    rows
}

fn render_dependency_row(edge: &HirModuleDependency) -> String {
    format!(
        "source={}:{}:{}:{}",
        edge.source_module_id,
        edge.kind.as_str(),
        edge.target_path.join("."),
        edge.alias.as_deref().unwrap_or("")
    )
}

fn canonical_dependency_path(package: &HirWorkspacePackage, path: &[String]) -> Vec<String> {
    let Some((first, suffix)) = path.split_first() else {
        return Vec::new();
    };
    let canonical_root = package
        .dependency_package_name(first)
        .unwrap_or(first)
        .to_string();
    let mut canonical = Vec::with_capacity(path.len());
    canonical.push(canonical_root);
    canonical.extend(suffix.iter().cloned());
    canonical
}

fn render_resolved_dependency_row(
    package: &HirWorkspacePackage,
    edge: &HirModuleDependency,
) -> String {
    format!(
        "source={}:{}:{}:{}",
        edge.source_module_id,
        edge.kind.as_str(),
        canonical_dependency_path(package, &edge.target_path).join("."),
        edge.alias.as_deref().unwrap_or("")
    )
}

fn resolved_direct_deps(package: &HirWorkspacePackage) -> Vec<String> {
    package
        .direct_dep_packages
        .values()
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn resolved_direct_dep_ids(package: &HirWorkspacePackage) -> Vec<String> {
    package
        .direct_dep_ids
        .values()
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn resolved_direct_dep_package_ids(package: &HirWorkspacePackage) -> BTreeMap<String, String> {
    package
        .direct_dep_packages
        .iter()
        .filter_map(|(visible_name, package_name)| {
            package
                .direct_dep_ids
                .get(visible_name)
                .map(|package_id| (package_name.clone(), package_id.clone()))
        })
        .collect()
}

fn resolved_direct_dep_display_names(package: &HirWorkspacePackage) -> BTreeMap<String, String> {
    package
        .direct_dep_ids
        .iter()
        .filter_map(|(visible_name, package_id)| {
            package
                .direct_dep_packages
                .get(visible_name)
                .map(|package_name| (package_id.clone(), package_name.clone()))
        })
        .collect()
}

fn encode_surface_text(text: &str) -> String {
    text.replace('\\', "\\\\").replace('\n', "\\n")
}

fn render_impl_surface_row(impl_decl: &arcana_hir::HirImplDecl) -> String {
    let methods = impl_decl
        .methods
        .iter()
        .map(|method| {
            format!(
                "{}:{}",
                method.kind.as_str(),
                encode_surface_text(&render_symbol_signature(method))
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "impl:target={}:trait={}:methods=[{}]",
        encode_surface_text(&impl_decl.target_type.render()),
        encode_surface_text(
            &impl_decl
                .trait_path
                .as_ref()
                .map(|path| path.render())
                .unwrap_or_default(),
        ),
        methods
    )
}

fn resolved_module_exported_surface_rows(
    workspace: &HirWorkspaceSummary,
    package: &HirWorkspacePackage,
    module: &HirModuleSummary,
) -> Vec<String> {
    let mut rows = module.summary_surface_rows();
    rows.extend(module.memory_specs.iter().filter_map(|spec| {
        render_memory_spec_surface_row(&lower_module_memory_spec_exec(spec)).ok()
    }));
    rows.extend(
        module
            .impls
            .iter()
            .filter(|&impl_decl| {
                impl_target_is_public_from_package(
                    workspace,
                    package,
                    module,
                    &impl_decl.target_type,
                )
            })
            .map(render_impl_surface_row),
    );
    rows.sort();
    rows.dedup();
    rows
}

fn quote_text(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

fn decode_row_string(text: &str) -> Result<String, String> {
    let inner = text
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .ok_or_else(|| format!("malformed source string literal `{text}`"))?;
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            let Some(next) = chars.next() else {
                return Err("unterminated escape in source string".to_string());
            };
            match next {
                '\\' => out.push('\\'),
                '"' => out.push('"'),
                'n' => out.push('\n'),
                't' => out.push('\t'),
                other => out.push(other),
            }
        } else {
            out.push(ch);
        }
    }
    Ok(out)
}

fn decode_source_string_literal(text: &str) -> Result<String, String> {
    let source = decode_row_string(text)?;
    if source.starts_with('"') && source.ends_with('"') && source.len() >= 2 {
        decode_row_string(&source)
    } else {
        Ok(source)
    }
}

fn symbol_has_builtin_foreword(symbol: &HirSymbol, name: &str) -> bool {
    symbol
        .forewords
        .iter()
        .any(|app| app.path.len() == 1 && app.name == name)
}

#[derive(Clone, Debug)]
struct IrResolvedForewordExport {
    exposed_name: Vec<String>,
    retention: IrForewordRetention,
    public: bool,
}

#[derive(Default)]
struct IrForewordRegistry {
    exports: BTreeMap<(String, String), IrResolvedForewordExport>,
}

fn lower_foreword_retention(retention: arcana_hir::HirForewordRetention) -> IrForewordRetention {
    match retention {
        arcana_hir::HirForewordRetention::Compile => IrForewordRetention::Compile,
        arcana_hir::HirForewordRetention::Tooling => IrForewordRetention::Tooling,
        arcana_hir::HirForewordRetention::Runtime => IrForewordRetention::Runtime,
    }
}

fn lower_foreword_arg(arg: &HirForewordArg) -> IrForewordArg {
    IrForewordArg {
        name: arg.name.clone(),
        value: arg.value.clone(),
        typed_value: match &arg.typed_value {
            arcana_hir::HirForewordArgValue::Raw(value) => IrForewordArgValue::Raw(value.clone()),
            arcana_hir::HirForewordArgValue::Bool(value) => IrForewordArgValue::Bool(*value),
            arcana_hir::HirForewordArgValue::Int(value) => IrForewordArgValue::Int(*value),
            arcana_hir::HirForewordArgValue::Str(value) => IrForewordArgValue::Str(value.clone()),
            arcana_hir::HirForewordArgValue::Symbol(value) => {
                IrForewordArgValue::Symbol(value.clone())
            }
            arcana_hir::HirForewordArgValue::Path(value) => IrForewordArgValue::Path(value.clone()),
        },
    }
}

fn lower_foreword_args(args: &[HirForewordArg]) -> Vec<IrForewordArg> {
    args.iter().map(lower_foreword_arg).collect()
}

fn lower_generated_by(generated_by: &arcana_hir::HirGeneratedByForeword) -> IrForewordGeneratedBy {
    IrForewordGeneratedBy {
        applied_name: generated_by.applied_name.clone(),
        resolved_name: generated_by.resolved_name.clone(),
        provider_package_id: generated_by.provider_package_id.clone(),
        owner_kind: generated_by.owner_kind.clone(),
        owner_path: generated_by.owner_path.clone(),
        retention: lower_foreword_retention(generated_by.retention),
        args: lower_foreword_args(&generated_by.args),
    }
}

fn impl_target_path(module_id: &str, impl_decl: &HirImplDecl) -> String {
    let target = impl_decl.target_type.to_string();
    let container = match &impl_decl.trait_path {
        Some(trait_path) => format!("{target}:{}", arcana_hir::render_hir_trait_ref(trait_path)),
        None => target,
    };
    format!("{module_id}::impl({container})")
}

fn build_ir_foreword_registry(workspace: &HirWorkspaceSummary) -> IrForewordRegistry {
    let mut registry = IrForewordRegistry::default();
    let mut local_defs = BTreeMap::<(String, String), arcana_hir::HirForewordDefinition>::new();
    for package in workspace.packages.values() {
        let package_name = &package.summary.package_name;
        for module in &package.summary.modules {
            for definition in &module.foreword_definitions {
                if definition.qualified_name.len() < 2
                    || definition.qualified_name[0] != *package_name
                {
                    continue;
                }
                local_defs.insert(
                    (
                        package.package_id.clone(),
                        definition.qualified_name[1..].join("."),
                    ),
                    definition.clone(),
                );
            }
        }
    }

    for package in workspace.packages.values() {
        for ((provider_package_id, tail), definition) in &local_defs {
            if provider_package_id != &package.package_id {
                continue;
            }
            registry.exports.insert(
                (package.package_id.clone(), tail.clone()),
                IrResolvedForewordExport {
                    exposed_name: definition.qualified_name.clone(),
                    retention: lower_foreword_retention(definition.retention),
                    public: definition.visibility == arcana_hir::HirForewordVisibility::Public,
                },
            );
        }
        for module in &package.summary.modules {
            for alias in &module.foreword_aliases {
                if alias.alias_name.len() < 2 || alias.source_name.len() < 2 {
                    continue;
                }
                let provider_package_id = if alias.source_name[0] == package.summary.package_name {
                    package.package_id.clone()
                } else if let Some(dep_id) = package.direct_dep_ids.get(&alias.source_name[0]) {
                    dep_id.clone()
                } else if let Some((alias_name, _)) = package
                    .direct_dep_packages
                    .iter()
                    .find(|(_, dep_name)| **dep_name == alias.source_name[0])
                {
                    let Some(dep_id) = package.direct_dep_ids.get(alias_name) else {
                        continue;
                    };
                    dep_id.clone()
                } else {
                    continue;
                };
                let Some(definition) = local_defs.get(&(
                    provider_package_id.clone(),
                    alias.source_name[1..].join("."),
                )) else {
                    continue;
                };
                if provider_package_id != package.package_id
                    && definition.visibility != arcana_hir::HirForewordVisibility::Public
                {
                    continue;
                }
                registry.exports.insert(
                    (package.package_id.clone(), alias.alias_name[1..].join(".")),
                    IrResolvedForewordExport {
                        exposed_name: alias.alias_name.clone(),
                        retention: lower_foreword_retention(definition.retention),
                        public: alias.kind == arcana_hir::HirForewordAliasKind::Reexport,
                    },
                );
            }
        }
    }
    registry
}

fn resolve_ir_foreword_export<'a>(
    package: &HirWorkspacePackage,
    app: &HirForewordApp,
    registry: &'a IrForewordRegistry,
) -> Option<&'a IrResolvedForewordExport> {
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

fn lower_foreword_index_entries(
    package: &HirWorkspacePackage,
    module_id: &str,
    apps: &[HirForewordApp],
    target_kind: &str,
    target_path: String,
    target_public: bool,
    target_generated_by: Option<&arcana_hir::HirGeneratedByForeword>,
    registry: &IrForewordRegistry,
    out: &mut Vec<IrForewordMetadata>,
) {
    for app in apps {
        let (qualified_name, retention) = if app.path.len() == 1 {
            (app.path.join("."), IrForewordRetention::Compile)
        } else if let Some(export) = resolve_ir_foreword_export(package, app, registry) {
            (export.exposed_name.join("."), export.retention.clone())
        } else {
            (app.path.join("."), IrForewordRetention::Compile)
        };
        out.push(IrForewordMetadata {
            entry_kind: IrForewordEntryKind::Attached,
            qualified_name,
            package_id: package.package_id.clone(),
            module_id: module_id.to_string(),
            target_kind: target_kind.to_string(),
            target_path: target_path.clone(),
            retention,
            args: lower_foreword_args(&app.args),
            public: target_public,
            generated_by: target_generated_by.map(lower_generated_by),
        });
    }
}

fn lower_generated_foreword_index_entry(
    package: &HirWorkspacePackage,
    module_id: &str,
    target_kind: &str,
    target_path: String,
    public: bool,
    generated_by: &arcana_hir::HirGeneratedByForeword,
    out: &mut Vec<IrForewordMetadata>,
) {
    out.push(IrForewordMetadata {
        entry_kind: IrForewordEntryKind::Generated,
        qualified_name: generated_by.resolved_name.clone(),
        package_id: package.package_id.clone(),
        module_id: module_id.to_string(),
        target_kind: target_kind.to_string(),
        target_path,
        retention: lower_foreword_retention(generated_by.retention),
        args: lower_foreword_args(&generated_by.args),
        public,
        generated_by: Some(lower_generated_by(generated_by)),
    });
}

fn lower_emitted_foreword_index_entry(
    package: &HirWorkspacePackage,
    module_id: &str,
    entry: &arcana_hir::HirEmittedForewordMetadata,
    out: &mut Vec<IrForewordMetadata>,
) {
    out.push(IrForewordMetadata {
        entry_kind: IrForewordEntryKind::Emitted,
        qualified_name: entry.qualified_name.clone(),
        package_id: package.package_id.clone(),
        module_id: module_id.to_string(),
        target_kind: entry.target_kind.clone(),
        target_path: entry.target_path.clone(),
        retention: lower_foreword_retention(entry.retention),
        args: lower_foreword_args(&entry.args),
        public: entry.public,
        generated_by: Some(lower_generated_by(&entry.generated_by)),
    });
}

fn lower_foreword_registration_row(
    row: &arcana_hir::HirForewordRegistrationRow,
) -> IrForewordRegistrationRow {
    IrForewordRegistrationRow {
        namespace: row.namespace.clone(),
        key: row.key.clone(),
        value: row.value.clone(),
        target_kind: row.target_kind.clone(),
        target_path: row.target_path.clone(),
        public: row.public,
        generated_by: lower_generated_by(&row.generated_by),
    }
}

fn lower_symbol_foreword_index_entries(
    package: &HirWorkspacePackage,
    module: &HirModuleSummary,
    symbol: &HirSymbol,
    target_kind: &str,
    public: bool,
    inherited_generated_by: Option<&arcana_hir::HirGeneratedByForeword>,
    registry: &IrForewordRegistry,
    out: &mut Vec<IrForewordMetadata>,
) {
    let symbol_target_path = format!("{}.{}", module.module_id, symbol.name);
    let target_generated_by = symbol.generated_by.as_ref().or(inherited_generated_by);
    if let Some(generated_by) = symbol.generated_by.as_ref() {
        lower_generated_foreword_index_entry(
            package,
            &module.module_id,
            target_kind,
            symbol_target_path.clone(),
            public,
            generated_by,
            out,
        );
    }
    lower_foreword_index_entries(
        package,
        &module.module_id,
        &symbol.forewords,
        target_kind,
        symbol_target_path.clone(),
        public,
        target_generated_by,
        registry,
        out,
    );
    for param in &symbol.params {
        lower_foreword_index_entries(
            package,
            &module.module_id,
            &param.forewords,
            "param",
            format!("{}.{}({})", module.module_id, symbol.name, param.name),
            public,
            target_generated_by,
            registry,
            out,
        );
    }
    match &symbol.body {
        HirSymbolBody::Record { fields } => {
            for field in fields {
                lower_foreword_index_entries(
                    package,
                    &module.module_id,
                    &field.forewords,
                    "field",
                    format!("{}.{}.{}", module.module_id, symbol.name, field.name),
                    public,
                    target_generated_by,
                    registry,
                    out,
                );
            }
        }
        HirSymbolBody::Object { fields, methods } => {
            for field in fields {
                lower_foreword_index_entries(
                    package,
                    &module.module_id,
                    &field.forewords,
                    "field",
                    format!("{}.{}.{}", module.module_id, symbol.name, field.name),
                    public,
                    target_generated_by,
                    registry,
                    out,
                );
            }
            for method in methods {
                lower_symbol_foreword_index_entries(
                    package,
                    module,
                    method,
                    "impl_method",
                    public,
                    target_generated_by,
                    registry,
                    out,
                );
            }
        }
        HirSymbolBody::Trait { methods, .. } => {
            for method in methods {
                lower_symbol_foreword_index_entries(
                    package,
                    module,
                    method,
                    "trait_method",
                    public,
                    target_generated_by,
                    registry,
                    out,
                );
            }
        }
        _ => {}
    }
}

fn lower_package_foreword_index(
    workspace: &HirWorkspaceSummary,
    package: &HirWorkspacePackage,
) -> Vec<IrForewordMetadata> {
    let registry = build_ir_foreword_registry(workspace);
    let mut index = Vec::new();
    for module in &package.summary.modules {
        for directive in &module.directives {
            let target_kind = match directive.kind {
                HirDirectiveKind::Import => "import",
                HirDirectiveKind::Use => "use",
                HirDirectiveKind::Reexport => "reexport",
            };
            lower_foreword_index_entries(
                package,
                &module.module_id,
                &directive.forewords,
                target_kind,
                format!("{}:{}", module.module_id, directive.path.join(".")),
                directive.kind == HirDirectiveKind::Reexport,
                None,
                &registry,
                &mut index,
            );
        }
        for entry in &module.emitted_foreword_metadata {
            lower_emitted_foreword_index_entry(package, &module.module_id, entry, &mut index);
        }
        for symbol in &module.symbols {
            lower_symbol_foreword_index_entries(
                package,
                module,
                symbol,
                symbol.kind.as_str(),
                symbol.exported,
                None,
                &registry,
                &mut index,
            );
        }
        for impl_decl in &module.impls {
            if let Some(generated_by) = impl_decl.generated_by.as_ref() {
                lower_generated_foreword_index_entry(
                    package,
                    &module.module_id,
                    "impl",
                    impl_target_path(&module.module_id, impl_decl),
                    false,
                    generated_by,
                    &mut index,
                );
            }
            for method in &impl_decl.methods {
                lower_symbol_foreword_index_entries(
                    package,
                    module,
                    method,
                    "impl_method",
                    false,
                    impl_decl.generated_by.as_ref(),
                    &registry,
                    &mut index,
                );
            }
        }
    }
    index.sort_by(|left, right| {
        left.qualified_name
            .cmp(&right.qualified_name)
            .then_with(|| left.module_id.cmp(&right.module_id))
            .then_with(|| left.target_path.cmp(&right.target_path))
            .then_with(|| left.target_kind.cmp(&right.target_kind))
            .then_with(|| left.entry_kind.as_str().cmp(right.entry_kind.as_str()))
    });
    index
}

fn lower_package_foreword_registrations(
    package: &HirWorkspacePackage,
) -> Vec<IrForewordRegistrationRow> {
    let mut rows = package
        .summary
        .modules
        .iter()
        .flat_map(|module| {
            module
                .foreword_registrations
                .iter()
                .map(lower_foreword_registration_row)
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        left.namespace
            .cmp(&right.namespace)
            .then_with(|| left.key.cmp(&right.key))
            .then_with(|| left.target_path.cmp(&right.target_path))
    });
    rows
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
    matches!(
        parse_binding_pattern(text),
        Some(BindingPattern::Pair(_, _))
    )
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

fn build_exec_member_chain(base: ExecExpr, members: &[&str]) -> ExecExpr {
    members.iter().fold(base, |expr, member| ExecExpr::Member {
        expr: Box::new(expr),
        member: (*member).to_string(),
    })
}

fn collect_binding_pattern_exec_lets(
    pattern: &BindingPattern,
    base_expr: &ExecExpr,
    path: &mut Vec<&'static str>,
    lets: &mut Vec<ExecStmt>,
    mutable: bool,
) {
    match pattern {
        BindingPattern::Name(name) => lets.push(ExecStmt::Let {
            binding_id: 0,
            mutable,
            name: name.clone(),
            value: build_exec_member_chain(base_expr.clone(), path),
        }),
        BindingPattern::Pair(left, right) => {
            path.push("0");
            collect_binding_pattern_exec_lets(left, base_expr, path, lets, mutable);
            path.pop();
            path.push("1");
            collect_binding_pattern_exec_lets(right, base_expr, path, lets, mutable);
            path.pop();
        }
    }
}

fn collect_typed_binding_pattern_exec_lets(
    pattern: &BindingPattern,
    ty: &HirType,
    base_expr: &ExecExpr,
    path: &mut Vec<&'static str>,
    lets: &mut Vec<ExecStmt>,
    mutable: bool,
    scope: &mut ResolvedRenderScope<'_>,
) -> Result<(), String> {
    match pattern {
        BindingPattern::Name(name) => {
            let binding_id = scope.value_scope.insert(name.clone(), ty.clone());
            lets.push(ExecStmt::Let {
                binding_id,
                mutable,
                name: name.clone(),
                value: build_exec_member_chain(base_expr.clone(), path),
            });
            Ok(())
        }
        BindingPattern::Pair(left, right) => {
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
            path.push("0");
            collect_typed_binding_pattern_exec_lets(
                left, left_ty, base_expr, path, lets, mutable, scope,
            )?;
            path.pop();
            path.push("1");
            collect_typed_binding_pattern_exec_lets(
                right, right_ty, base_expr, path, lets, mutable, scope,
            )?;
            path.pop();
            Ok(())
        }
    }
}

fn split_simple_path(text: &str) -> Option<Vec<String>> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    let segments = trimmed
        .split('.')
        .map(str::trim)
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    (!segments.is_empty() && segments.iter().all(|segment| is_identifier_text(segment)))
        .then_some(segments)
}

fn canonical_impl_trait_path(path: &arcana_hir::HirTraitRef) -> Vec<String> {
    path.path.segments.clone()
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

fn flatten_callable_expr_path(expr: &HirExpr) -> Option<Vec<String>> {
    match expr {
        HirExpr::GenericApply { expr, .. } => flatten_callable_expr_path(expr),
        _ => flatten_member_expr_path(expr),
    }
}

fn resolved_symbol_path(symbol_ref: arcana_hir::HirResolvedSymbolRef<'_>) -> Vec<String> {
    let mut path = symbol_ref
        .module_id
        .split('.')
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    path.push(symbol_ref.symbol.name.clone());
    path
}

fn resolved_symbol_routine_key(symbol_ref: &arcana_hir::HirResolvedSymbolRef<'_>) -> String {
    routine_key_for_symbol(symbol_ref.module_id, symbol_ref.symbol_index)
}

fn format_method_ambiguity(
    ty: &HirType,
    method_name: &str,
    candidates: &[ResolvedMethod<'_>],
) -> String {
    let rendered = candidates
        .iter()
        .map(|candidate| {
            format!(
                "{} [{}]",
                render_symbol_signature(candidate.method),
                candidate.routine_key
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "bare-method qualifier `{method_name}` on `{}` is ambiguous; candidates: {rendered}",
        ty.render()
    )
}

#[derive(Clone, Debug)]
struct ResolvedMethod<'a> {
    module_id: String,
    method: &'a HirSymbol,
    routine_key: String,
    trait_path: Option<Vec<String>>,
}

#[derive(Clone, Debug)]
struct ResolvedPhraseTarget {
    path: Vec<String>,
    routine_key: Option<String>,
    dynamic_dispatch: Option<ExecDynamicDispatch>,
}

struct ResolvedOwnerActivation<'a> {
    owner_path: Vec<String>,
    owner_local_name: String,
    objects: Vec<(String, HirType)>,
    context: Option<&'a HirExpr>,
}

fn lookup_method_resolution_for_type<'a>(
    scope: &'a ResolvedRenderScope<'a>,
    workspace: &'a HirWorkspaceSummary,
    ty: &HirType,
    method_name: &str,
) -> Result<Option<ResolvedMethod<'a>>, String> {
    let candidates =
        lookup_method_candidates_for_hir_type(workspace, scope.resolved_module, ty, method_name)
            .into_iter()
            .map(|candidate| ResolvedMethod {
                module_id: candidate.module_id.to_string(),
                method: candidate.symbol,
                routine_key: candidate.routine_key,
                trait_path: candidate.trait_path,
            })
            .collect::<Vec<_>>();
    match candidates.as_slice() {
        [] => Ok(None),
        [resolved] => Ok(Some(resolved.clone())),
        _ => Err(format_method_ambiguity(ty, method_name, &candidates)),
    }
}

fn lookup_trait_method_resolution_from_where_clause<'a>(
    scope: &'a ResolvedRenderScope<'a>,
    ty: &HirType,
    method_name: &str,
) -> Result<Vec<ResolvedMethod<'a>>, String> {
    let Some(where_clause) = scope.current_where_clause.as_ref() else {
        return Ok(Vec::new());
    };
    let mut candidates = Vec::new();
    for predicate in &where_clause.predicates {
        let HirPredicate::TraitBound { trait_ref, .. } = predicate else {
            continue;
        };
        if !trait_ref.args.iter().any(|arg| {
            let mut substitutions = HirTypeSubstitutions::default();
            hir_type_matches(arg, ty, &scope.type_bindings, &mut substitutions)
        }) {
            continue;
        }
        let Some(symbol_ref) = lookup_symbol_path(
            scope.workspace,
            scope.resolved_module,
            &trait_ref.path.segments,
        ) else {
            continue;
        };
        let HirSymbolBody::Trait { methods, .. } = &symbol_ref.symbol.body else {
            continue;
        };
        if let Some(method) = methods.iter().find(|method| method.name == method_name) {
            candidates.push(ResolvedMethod {
                module_id: symbol_ref.module_id.to_string(),
                method,
                routine_key: String::new(),
                trait_path: Some(trait_ref.path.segments.clone()),
            });
        }
    }
    Ok(candidates)
}

fn lower_symbol_routine_type(ty: &HirType) -> IrRoutineType {
    IrRoutineType::from_hir(ty)
}

fn lower_resolved_routine_type(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    ty: &HirType,
) -> IrRoutineType {
    current_workspace_package_for_module(workspace, resolved_module)
        .and_then(|package| {
            package
                .module(&resolved_module.module_id)
                .map(|module| (package, module))
        })
        .map(|(package, module)| {
            IrRoutineType::from_hir(&canonicalize_hir_type_in_module(
                workspace, package, module, ty,
            ))
        })
        .unwrap_or_else(|| IrRoutineType::from_hir(ty))
}

fn lower_resolved_routine_params(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    params: &[arcana_hir::HirParam],
) -> Vec<IrRoutineParam> {
    params
        .iter()
        .map(|param| IrRoutineParam {
            binding_id: 0,
            mode: param.mode.map(|mode| mode.as_str().to_string()),
            name: param.name.clone(),
            ty: lower_resolved_routine_type(workspace, resolved_module, &param.ty),
        })
        .collect()
}

fn lower_native_callback_signature(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    callback: &arcana_hir::HirNativeCallbackDecl,
) -> Result<(Vec<IrRoutineParam>, Option<IrRoutineType>), String> {
    let explicit_params =
        lower_resolved_routine_params(workspace, resolved_module, &callback.params);
    let explicit_return = callback
        .return_type
        .as_ref()
        .map(|ty| lower_resolved_routine_type(workspace, resolved_module, ty));
    let Some(callback_type) = &callback.callback_type else {
        return Ok((explicit_params, explicit_return));
    };
    let HirTypeKind::Path(path) = &callback_type.kind else {
        return Err(format!(
            "native callback `{}` callback type `{}` must be a path",
            callback.name,
            callback_type.render()
        ));
    };
    let Some(decl_ref) = lookup_shackle_decl_path(workspace, resolved_module, &path.segments)
    else {
        return Err(format!(
            "native callback `{}` callback type `{}` does not resolve to a visible shackle callback",
            callback.name,
            callback_type.render()
        ));
    };
    if decl_ref.decl.kind.as_str() != "callback" {
        return Err(format!(
            "native callback `{}` callback type `{}` must resolve to a shackle callback",
            callback.name,
            callback_type.render()
        ));
    }
    let params = if explicit_params.is_empty() {
        lower_resolved_routine_params(workspace, resolved_module, &decl_ref.decl.params)
    } else {
        explicit_params
    };
    let return_type = explicit_return.or_else(|| {
        decl_ref
            .decl
            .return_type
            .as_ref()
            .map(|ty| lower_resolved_routine_type(workspace, resolved_module, ty))
    });
    Ok((params, return_type))
}

fn lower_routine_params(symbol: &HirSymbol) -> Vec<IrRoutineParam> {
    symbol
        .params
        .iter()
        .map(|param| IrRoutineParam {
            binding_id: 0,
            mode: param.mode.map(|mode| mode.as_str().to_string()),
            name: param.name.clone(),
            ty: lower_symbol_routine_type(&param.ty),
        })
        .collect()
}

fn lower_routine_params_resolved(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    symbol: &HirSymbol,
    scope: &ResolvedRenderScope<'_>,
) -> Vec<IrRoutineParam> {
    symbol
        .params
        .iter()
        .map(|param| IrRoutineParam {
            binding_id: scope.value_scope.binding_id_of(&param.name).unwrap_or(0),
            mode: param.mode.map(|mode| mode.as_str().to_string()),
            name: param.name.clone(),
            ty: lower_resolved_routine_type(workspace, resolved_module, &param.ty),
        })
        .collect()
}

fn lower_behavior_attrs(symbol: &HirSymbol) -> BTreeMap<String, String> {
    symbol
        .behavior_attrs
        .iter()
        .map(|attr| (attr.name.clone(), attr.value.clone()))
        .collect()
}

fn infer_iterable_binding_type(
    scope: &ResolvedRenderScope<'_>,
    iterable: &HirExpr,
) -> Option<HirType> {
    let iterable_ty = infer_expr_hir_type(scope, iterable)?;
    match &iterable_ty.kind {
        HirTypeKind::Path(path) if hir_path_matches(path, &["RangeInt"]) => {
            Some(simple_hir_type("Int"))
        }
        HirTypeKind::Apply { base, args }
            if hir_path_matches_any(
                base,
                &[
                    &["List"],
                    &["Array"],
                    &["std", "collections", "list", "List"],
                    &["std", "collections", "array", "Array"],
                ],
            ) =>
        {
            args.first().cloned()
        }
        HirTypeKind::Apply { base, args }
            if hir_path_matches_any(base, &[&["Map"], &["std", "collections", "map", "Map"]]) =>
        {
            match (args.first(), args.get(1)) {
                (Some(key), Some(value)) => Some(pair_hir_type(key.clone(), value.clone())),
                _ => None,
            }
        }
        _ => None,
    }
}

fn resolve_qualified_phrase_target_path(
    scope: &ResolvedRenderScope<'_>,
    subject: &HirExpr,
    qualifier_kind: arcana_hir::HirQualifiedPhraseQualifierKind,
    qualifier: &str,
) -> Option<ResolvedPhraseTarget> {
    match qualifier_kind {
        arcana_hir::HirQualifiedPhraseQualifierKind::Call
        | arcana_hir::HirQualifiedPhraseQualifierKind::Weave
        | arcana_hir::HirQualifiedPhraseQualifierKind::Split => {
            let path = flatten_callable_expr_path(subject)?;
            lookup_symbol_path(scope.workspace, scope.resolved_module, &path).map(|resolved| {
                let routine_key = resolved_symbol_routine_key(&resolved);
                ResolvedPhraseTarget {
                    path: resolved_symbol_path(resolved),
                    routine_key: Some(routine_key),
                    dynamic_dispatch: None,
                }
            })
        }
        arcana_hir::HirQualifiedPhraseQualifierKind::NamedPath => {
            let path = split_simple_path(qualifier).filter(|path| path.len() > 1)?;
            let resolved = lookup_symbol_path(scope.workspace, scope.resolved_module, &path)?;
            let routine_key = resolved_symbol_routine_key(&resolved);
            Some(ResolvedPhraseTarget {
                path: resolved_symbol_path(resolved),
                routine_key: Some(routine_key),
                dynamic_dispatch: None,
            })
        }
        _ => None,
    }
}

fn resolve_bare_method_target(
    scope: &ResolvedRenderScope<'_>,
    subject: &HirExpr,
    qualifier: &str,
) -> Option<ResolvedPhraseTarget> {
    let subject_ty = infer_receiver_expr_type(
        scope.workspace,
        scope.resolved_module,
        &scope.value_scope,
        subject,
    )?;
    match lookup_method_resolution_for_type(scope, scope.workspace, &subject_ty, qualifier) {
        Ok(Some(resolved)) => {
            let mut path = resolved
                .module_id
                .split('.')
                .map(ToString::to_string)
                .collect::<Vec<_>>();
            path.push(resolved.method.name.clone());
            return Some(ResolvedPhraseTarget {
                path,
                routine_key: Some(resolved.routine_key),
                dynamic_dispatch: None,
            });
        }
        Ok(None) => {}
        Err(message) => {
            scope.note_error(message);
            return None;
        }
    }
    match lookup_trait_method_resolution_from_where_clause(scope, &subject_ty, qualifier) {
        Ok(candidates) => {
            if candidates.len() > 1 {
                scope.note_error(format!(
                    "bare-method qualifier `{qualifier}` on `{}` is ambiguous across trait bounds",
                    subject_ty.render()
                ));
                return None;
            }
            let resolved = candidates.into_iter().next()?;
            let mut path = resolved
                .module_id
                .split('.')
                .map(ToString::to_string)
                .collect::<Vec<_>>();
            path.push(resolved.method.name.clone());
            Some(ResolvedPhraseTarget {
                path,
                routine_key: None,
                dynamic_dispatch: resolved
                    .trait_path
                    .map(|trait_path| ExecDynamicDispatch::TraitMethod { trait_path }),
            })
        }
        Err(message) => {
            scope.note_error(message);
            None
        }
    }
}

fn infer_expr_hir_type(scope: &ResolvedRenderScope<'_>, expr: &HirExpr) -> Option<HirType> {
    if let HirExpr::MemberAccess { expr, member } = expr
        && let HirExpr::Path { segments } = expr.as_ref()
        && segments.len() == 1
        && scope.value_scope.contains(&segments[0])
        && let Some(ty) = scope.value_scope.owner_member_type(&segments[0], member)
    {
        return Some(ty.clone());
    }
    if let Some(inferred) = infer_receiver_expr_type(
        scope.workspace,
        scope.resolved_module,
        &scope.value_scope,
        expr,
    ) {
        return Some(inferred);
    }
    match expr {
        HirExpr::ConstructRegion(region) => flatten_callable_expr_path(&region.target)
            .and_then(|path| resolve_construct_result_type_for_scope(scope, &path)),
        HirExpr::RecordRegion(region) => flatten_callable_expr_path(&region.target)
            .and_then(|path| resolve_record_result_type_for_scope(scope, &path)),
        HirExpr::Pair { left, right } => Some(pair_hir_type(
            infer_expr_hir_type(scope, left)?,
            infer_expr_hir_type(scope, right)?,
        )),
        _ => None,
    }
}

fn infer_headed_payload_binding_type(
    scope: &ResolvedRenderScope<'_>,
    expr: &HirExpr,
) -> Option<HirType> {
    let ty = infer_expr_hir_type(scope, expr)?;
    if let Some(payload) = type_option_payload_for_construct(&ty) {
        Some(payload)
    } else {
        type_result_payloads_for_construct(&ty).map(|(ok, _)| ok)
    }
}

#[derive(Clone, Debug)]
enum ResolvedConstructTargetShape {
    Record(BTreeMap<String, HirType>),
    Variant(HirType),
}

fn canonicalize_scope_hir_type(scope: &ResolvedRenderScope<'_>, ty: &HirType) -> Option<HirType> {
    let package = current_workspace_package_for_module(scope.workspace, scope.resolved_module)?;
    let module = package.module(&scope.resolved_module.module_id)?;
    Some(canonicalize_hir_type_in_module(
        scope.workspace,
        package,
        module,
        ty,
    ))
}

fn type_option_payload_for_construct(ty: &HirType) -> Option<HirType> {
    let HirTypeKind::Apply { base, args } = &ty.kind else {
        return None;
    };
    (base.segments.last().map(String::as_str) == Some("Option") && args.len() == 1)
        .then(|| args[0].clone())
}

fn type_result_payloads_for_construct(ty: &HirType) -> Option<(HirType, HirType)> {
    let HirTypeKind::Apply { base, args } = &ty.kind else {
        return None;
    };
    (base.segments.last().map(String::as_str) == Some("Result") && args.len() == 2)
        .then(|| (args[0].clone(), args[1].clone()))
}

fn resolve_construct_target_shape_for_scope(
    scope: &ResolvedRenderScope<'_>,
    path: &[String],
) -> Option<ResolvedConstructTargetShape> {
    if let Some(resolved) = lookup_symbol_path(scope.workspace, scope.resolved_module, path)
        && let HirSymbolBody::Record { fields } = &resolved.symbol.body
    {
        let mut lowered = BTreeMap::new();
        for field in fields {
            lowered.insert(
                field.name.clone(),
                canonicalize_scope_hir_type(scope, &field.ty)?,
            );
        }
        return Some(ResolvedConstructTargetShape::Record(lowered));
    }
    let (variant_name, enum_path) = path.split_last()?;
    let resolved = lookup_symbol_path(scope.workspace, scope.resolved_module, enum_path)?;
    let HirSymbolBody::Enum { variants } = &resolved.symbol.body else {
        return None;
    };
    let variant = variants
        .iter()
        .find(|variant| variant.name == *variant_name)?;
    Some(ResolvedConstructTargetShape::Variant(
        canonicalize_scope_hir_type(scope, variant.payload.as_ref()?)?,
    ))
}

fn resolve_record_target_fields_for_scope(
    scope: &ResolvedRenderScope<'_>,
    path: &[String],
) -> Option<BTreeMap<String, HirType>> {
    let resolved = lookup_symbol_path(scope.workspace, scope.resolved_module, path)?;
    let HirSymbolBody::Record { fields } = &resolved.symbol.body else {
        return None;
    };
    let mut lowered = BTreeMap::new();
    for field in fields {
        lowered.insert(
            field.name.clone(),
            canonicalize_scope_hir_type(scope, &field.ty)?,
        );
    }
    Some(lowered)
}

fn resolve_record_fields_for_hir_type(
    scope: &ResolvedRenderScope<'_>,
    ty: &HirType,
) -> Option<BTreeMap<String, HirType>> {
    let path = match &ty.kind {
        HirTypeKind::Path(path) | HirTypeKind::Apply { base: path, .. } => &path.segments,
        _ => return None,
    };
    resolve_record_target_fields_for_scope(scope, path)
}

fn canonical_scope_type_from_path(scope: &ResolvedRenderScope<'_>, path: &[String]) -> HirType {
    let segments = lookup_symbol_path(scope.workspace, scope.resolved_module, path)
        .map(resolved_symbol_path)
        .unwrap_or_else(|| path.to_vec());
    HirType {
        kind: HirTypeKind::Path(HirPath {
            segments,
            span: Default::default(),
        }),
        span: Default::default(),
    }
}

fn resolve_construct_result_type_for_scope(
    scope: &ResolvedRenderScope<'_>,
    path: &[String],
) -> Option<HirType> {
    resolve_construct_target_shape_for_scope(scope, path)?;
    let canonical_path =
        if lookup_symbol_path(scope.workspace, scope.resolved_module, path).is_some() {
            path.to_vec()
        } else {
            path[..path.len().checked_sub(1)?].to_vec()
        };
    Some(canonical_scope_type_from_path(scope, &canonical_path))
}

fn resolve_record_result_type_for_scope(
    scope: &ResolvedRenderScope<'_>,
    path: &[String],
) -> Option<HirType> {
    resolve_record_target_fields_for_scope(scope, path)?;
    Some(canonical_scope_type_from_path(scope, path))
}

fn collect_record_copied_fields(
    scope: &ResolvedRenderScope<'_>,
    region: &arcana_hir::HirRecordRegion,
) -> Vec<String> {
    let Some(target_path) = flatten_callable_expr_path(&region.target) else {
        return Vec::new();
    };
    let Some(target_fields) = resolve_record_target_fields_for_scope(scope, &target_path) else {
        return Vec::new();
    };
    let Some(base) = &region.base else {
        return Vec::new();
    };
    let Some(base_ty) =
        infer_expr_hir_type(scope, base).and_then(|ty| canonicalize_scope_hir_type(scope, &ty))
    else {
        return Vec::new();
    };
    let Some(base_fields) = resolve_record_fields_for_hir_type(scope, &base_ty) else {
        return Vec::new();
    };
    let explicit = region
        .lines
        .iter()
        .map(|line| line.name.as_str())
        .collect::<BTreeSet<_>>();
    target_fields
        .into_iter()
        .filter_map(|(field, ty)| {
            (!explicit.contains(field.as_str())
                && base_fields
                    .get(&field)
                    .is_some_and(|base_ty| base_ty == &ty))
            .then_some(field)
        })
        .collect()
}

fn infer_construct_contribution_mode(
    scope: &ResolvedRenderScope<'_>,
    region: &arcana_hir::HirConstructRegion,
    line: &arcana_hir::HirConstructLine,
) -> ExecConstructContributionMode {
    let Some(target_path) = flatten_callable_expr_path(&region.target) else {
        return ExecConstructContributionMode::Direct;
    };
    let Some(expected_ty) = (match resolve_construct_target_shape_for_scope(scope, &target_path) {
        Some(ResolvedConstructTargetShape::Record(fields)) => fields.get(&line.name).cloned(),
        Some(ResolvedConstructTargetShape::Variant(payload)) if line.name == "payload" => {
            Some(payload)
        }
        _ => None,
    }) else {
        return ExecConstructContributionMode::Direct;
    };
    let expected_key = expected_ty.render();
    let Some(actual_ty) = infer_expr_hir_type(scope, &line.value)
        .and_then(|ty| canonicalize_scope_hir_type(scope, &ty))
    else {
        return ExecConstructContributionMode::Direct;
    };
    if actual_ty.render() == expected_key {
        return ExecConstructContributionMode::Direct;
    }
    if type_option_payload_for_construct(&actual_ty)
        .is_some_and(|payload| payload.render() == expected_key)
    {
        return ExecConstructContributionMode::OptionPayload;
    }
    if type_result_payloads_for_construct(&actual_ty)
        .is_some_and(|(ok, _)| ok.render() == expected_key)
    {
        return ExecConstructContributionMode::ResultPayload;
    }
    ExecConstructContributionMode::Direct
}

fn infer_record_contribution_mode(
    scope: &ResolvedRenderScope<'_>,
    region: &arcana_hir::HirRecordRegion,
    line: &arcana_hir::HirConstructLine,
) -> ExecConstructContributionMode {
    let Some(target_path) = flatten_callable_expr_path(&region.target) else {
        return ExecConstructContributionMode::Direct;
    };
    let Some(expected_ty) = resolve_record_target_fields_for_scope(scope, &target_path)
        .and_then(|fields| fields.get(&line.name).cloned())
    else {
        return ExecConstructContributionMode::Direct;
    };
    let expected_key = expected_ty.render();
    let Some(actual_ty) = infer_expr_hir_type(scope, &line.value)
        .and_then(|ty| canonicalize_scope_hir_type(scope, &ty))
    else {
        return ExecConstructContributionMode::Direct;
    };
    if actual_ty.render() == expected_key {
        return ExecConstructContributionMode::Direct;
    }
    if type_option_payload_for_construct(&actual_ty)
        .is_some_and(|payload| payload.render() == expected_key)
    {
        return ExecConstructContributionMode::OptionPayload;
    }
    if type_result_payloads_for_construct(&actual_ty)
        .is_some_and(|(ok, _)| ok.render() == expected_key)
    {
        return ExecConstructContributionMode::ResultPayload;
    }
    ExecConstructContributionMode::Direct
}

fn lower_rollup(rollup: &HirCleanupFooter) -> ExecCleanupFooter {
    ExecCleanupFooter {
        kind: rollup.kind.as_str().to_string(),
        binding_id: 0,
        subject: rollup.subject.clone(),
        handler_path: rollup.handler_path.clone(),
        resolved_routine: None,
    }
}

fn lower_phrase_qualifier_kind(
    qualifier_kind: arcana_hir::HirQualifiedPhraseQualifierKind,
) -> ExecPhraseQualifierKind {
    match qualifier_kind {
        arcana_hir::HirQualifiedPhraseQualifierKind::Call => ExecPhraseQualifierKind::Call,
        arcana_hir::HirQualifiedPhraseQualifierKind::Try => ExecPhraseQualifierKind::Try,
        arcana_hir::HirQualifiedPhraseQualifierKind::Apply => ExecPhraseQualifierKind::Apply,
        arcana_hir::HirQualifiedPhraseQualifierKind::AwaitApply => {
            ExecPhraseQualifierKind::AwaitApply
        }
        arcana_hir::HirQualifiedPhraseQualifierKind::Await => ExecPhraseQualifierKind::Await,
        arcana_hir::HirQualifiedPhraseQualifierKind::Weave => ExecPhraseQualifierKind::Weave,
        arcana_hir::HirQualifiedPhraseQualifierKind::Split => ExecPhraseQualifierKind::Split,
        arcana_hir::HirQualifiedPhraseQualifierKind::Must => ExecPhraseQualifierKind::Must,
        arcana_hir::HirQualifiedPhraseQualifierKind::Fallback => ExecPhraseQualifierKind::Fallback,
        arcana_hir::HirQualifiedPhraseQualifierKind::BareMethod => {
            ExecPhraseQualifierKind::BareMethod
        }
        arcana_hir::HirQualifiedPhraseQualifierKind::NamedPath => {
            ExecPhraseQualifierKind::NamedPath
        }
    }
}

fn lower_chain_connector(connector: HirChainConnector) -> ExecChainConnector {
    match connector {
        HirChainConnector::Forward => ExecChainConnector::Forward,
        HirChainConnector::Reverse => ExecChainConnector::Reverse,
    }
}

fn lower_chain_introducer(introducer: HirChainIntroducer) -> ExecChainIntroducer {
    match introducer {
        HirChainIntroducer::Forward => ExecChainIntroducer::Forward,
        HirChainIntroducer::Reverse => ExecChainIntroducer::Reverse,
    }
}

fn lower_unary_op(op: HirUnaryOp) -> ExecUnaryOp {
    match op {
        HirUnaryOp::Neg => ExecUnaryOp::Neg,
        HirUnaryOp::Not => ExecUnaryOp::Not,
        HirUnaryOp::BitNot => ExecUnaryOp::BitNot,
        HirUnaryOp::CapabilityRead => ExecUnaryOp::CapabilityRead,
        HirUnaryOp::CapabilityEdit => ExecUnaryOp::CapabilityEdit,
        HirUnaryOp::CapabilityTake => ExecUnaryOp::CapabilityTake,
        HirUnaryOp::CapabilityHold => ExecUnaryOp::CapabilityHold,
        HirUnaryOp::Deref => ExecUnaryOp::Deref,
        HirUnaryOp::Weave => ExecUnaryOp::Weave,
        HirUnaryOp::Split => ExecUnaryOp::Split,
    }
}

fn lower_binary_op(op: HirBinaryOp) -> ExecBinaryOp {
    match op {
        HirBinaryOp::Or => ExecBinaryOp::Or,
        HirBinaryOp::And => ExecBinaryOp::And,
        HirBinaryOp::EqEq => ExecBinaryOp::EqEq,
        HirBinaryOp::NotEq => ExecBinaryOp::NotEq,
        HirBinaryOp::Lt => ExecBinaryOp::Lt,
        HirBinaryOp::LtEq => ExecBinaryOp::LtEq,
        HirBinaryOp::Gt => ExecBinaryOp::Gt,
        HirBinaryOp::GtEq => ExecBinaryOp::GtEq,
        HirBinaryOp::BitOr => ExecBinaryOp::BitOr,
        HirBinaryOp::BitXor => ExecBinaryOp::BitXor,
        HirBinaryOp::BitAnd => ExecBinaryOp::BitAnd,
        HirBinaryOp::Shl => ExecBinaryOp::Shl,
        HirBinaryOp::Shr => ExecBinaryOp::Shr,
        HirBinaryOp::Add => ExecBinaryOp::Add,
        HirBinaryOp::Sub => ExecBinaryOp::Sub,
        HirBinaryOp::Mul => ExecBinaryOp::Mul,
        HirBinaryOp::Div => ExecBinaryOp::Div,
        HirBinaryOp::Mod => ExecBinaryOp::Mod,
    }
}

fn lower_assign_op(op: HirAssignOp) -> ExecAssignOp {
    match op {
        HirAssignOp::Assign => ExecAssignOp::Assign,
        HirAssignOp::AddAssign => ExecAssignOp::AddAssign,
        HirAssignOp::SubAssign => ExecAssignOp::SubAssign,
        HirAssignOp::MulAssign => ExecAssignOp::MulAssign,
        HirAssignOp::DivAssign => ExecAssignOp::DivAssign,
        HirAssignOp::ModAssign => ExecAssignOp::ModAssign,
        HirAssignOp::BitAndAssign => ExecAssignOp::BitAndAssign,
        HirAssignOp::BitOrAssign => ExecAssignOp::BitOrAssign,
        HirAssignOp::BitXorAssign => ExecAssignOp::BitXorAssign,
        HirAssignOp::ShlAssign => ExecAssignOp::ShlAssign,
        HirAssignOp::ShrAssign => ExecAssignOp::ShrAssign,
    }
}

fn lower_match_pattern_exec(pattern: &HirMatchPattern) -> ExecMatchPattern {
    match pattern {
        HirMatchPattern::Wildcard => ExecMatchPattern::Wildcard,
        HirMatchPattern::Literal { text } => ExecMatchPattern::Literal(text.clone()),
        HirMatchPattern::Name { text } => {
            if text.contains('.') {
                return ExecMatchPattern::Variant {
                    path: text.clone(),
                    args: Vec::new(),
                };
            }
            ExecMatchPattern::Name(text.clone())
        }
        HirMatchPattern::Variant { path, args } => ExecMatchPattern::Variant {
            path: path.clone(),
            args: args.iter().map(lower_match_pattern_exec).collect(),
        },
    }
}

fn lower_subject_match_pattern_exec_resolved(
    pattern: &HirMatchPattern,
    subject: &HirExpr,
    scope: &ResolvedRenderScope<'_>,
) -> ExecMatchPattern {
    match pattern {
        HirMatchPattern::Wildcard => ExecMatchPattern::Wildcard,
        HirMatchPattern::Literal { text } => ExecMatchPattern::Literal(text.clone()),
        HirMatchPattern::Name { text } => {
            if text.contains('.')
                || match_name_resolves_to_zero_payload_variant(
                    scope.workspace,
                    scope.resolved_module,
                    &scope.value_scope,
                    subject,
                    text,
                )
            {
                return ExecMatchPattern::Variant {
                    path: text.clone(),
                    args: Vec::new(),
                };
            }
            ExecMatchPattern::Name(text.clone())
        }
        HirMatchPattern::Variant { path, args } => ExecMatchPattern::Variant {
            path: path.clone(),
            args: args.iter().map(lower_match_pattern_exec).collect(),
        },
    }
}

fn lower_phrase_arg_exec(arg: &HirPhraseArg) -> ExecPhraseArg {
    match arg {
        HirPhraseArg::Positional(expr) => ExecPhraseArg {
            name: None,
            value: lower_exec_expr(expr),
        },
        HirPhraseArg::Named { name, value } => ExecPhraseArg {
            name: Some(name.clone()),
            value: lower_exec_expr(value),
        },
    }
}

fn lower_phrase_arg_exec_resolved(
    arg: &HirPhraseArg,
    scope: &ResolvedRenderScope<'_>,
) -> ExecPhraseArg {
    match arg {
        HirPhraseArg::Positional(expr) => ExecPhraseArg {
            name: None,
            value: lower_exec_expr_resolved(expr, scope),
        },
        HirPhraseArg::Named { name, value } => ExecPhraseArg {
            name: Some(name.clone()),
            value: lower_exec_expr_resolved(value, scope),
        },
    }
}

fn lower_header_attachment_exec(attachment: &HirHeaderAttachment) -> ExecHeaderAttachment {
    match attachment {
        HirHeaderAttachment::Named { name, value, .. } => ExecHeaderAttachment::Named {
            name: name.clone(),
            value: lower_exec_expr(value),
        },
        HirHeaderAttachment::Chain { expr, .. } => ExecHeaderAttachment::Chain {
            expr: lower_exec_expr(expr),
        },
    }
}

fn lower_header_attachment_exec_resolved(
    attachment: &HirHeaderAttachment,
    scope: &ResolvedRenderScope<'_>,
) -> ExecHeaderAttachment {
    match attachment {
        HirHeaderAttachment::Named { name, value, .. } => ExecHeaderAttachment::Named {
            name: name.clone(),
            value: lower_exec_expr_resolved(value, scope),
        },
        HirHeaderAttachment::Chain { expr, .. } => ExecHeaderAttachment::Chain {
            expr: lower_exec_expr_resolved(expr, scope),
        },
    }
}

fn resolve_owner_activation_expr<'a>(
    scope: &ResolvedRenderScope<'_>,
    expr: &'a HirExpr,
) -> Result<Option<ResolvedOwnerActivation<'a>>, String> {
    let HirExpr::QualifiedPhrase {
        subject,
        args,
        qualifier_kind,
        qualifier: _,
        ..
    } = expr
    else {
        return Ok(None);
    };
    if *qualifier_kind != arcana_hir::HirQualifiedPhraseQualifierKind::Call {
        return Ok(None);
    }
    let Some(path) = flatten_callable_expr_path(subject) else {
        return Ok(None);
    };
    let Some(resolved) = lookup_symbol_path(scope.workspace, scope.resolved_module, &path) else {
        return Ok(None);
    };
    if resolved.symbol.kind != HirSymbolKind::Owner {
        return Ok(None);
    }
    if args
        .iter()
        .any(|arg| matches!(arg, HirPhraseArg::Named { .. }))
    {
        return Err(format!(
            "owner activation `{}` does not support named arguments",
            path.join(".")
        ));
    }
    if args.len() > 1 {
        return Err(format!(
            "owner activation `{}` accepts at most one context argument",
            path.join(".")
        ));
    }
    let HirSymbolBody::Owner {
        objects,
        context_type,
        ..
    } = &resolved.symbol.body
    else {
        return Ok(None);
    };
    if context_type.is_some() && args.is_empty() {
        return Err(format!(
            "owner activation `{}` requires exactly one context argument",
            path.join(".")
        ));
    }
    if context_type.is_none() && !args.is_empty() {
        return Err(format!(
            "owner activation `{}` does not use an activation context",
            path.join(".")
        ));
    }
    Ok(Some(ResolvedOwnerActivation {
        owner_path: resolved_symbol_path(resolved),
        owner_local_name: resolved.symbol.name.clone(),
        objects: objects
            .iter()
            .map(|object| {
                (
                    object.local_name.clone(),
                    HirType {
                        kind: HirTypeKind::Path(HirPath {
                            segments: object.type_path.clone(),
                            span: object.span,
                        }),
                        span: object.span,
                    },
                )
            })
            .collect(),
        context: args.first().and_then(|arg| match arg {
            HirPhraseArg::Positional(expr) => Some(expr),
            HirPhraseArg::Named { .. } => None,
        }),
    }))
}

fn has_bare_cleanup_rollup(cleanup_footers: &[HirCleanupFooter]) -> bool {
    cleanup_footers
        .iter()
        .any(|rollup| rollup.subject.is_empty())
}

#[derive(Clone, Debug)]
struct CleanupBindingCandidate {
    name: String,
    binding_id: u64,
    ty: HirType,
}

#[derive(Clone, Debug, Default)]
struct LoweredExecBlock {
    statements: Vec<ExecStmt>,
    cleanup_bindings: Vec<CleanupBindingCandidate>,
}

fn targeted_cleanup_cleanup_footers(
    cleanup_footers: &[HirCleanupFooter],
) -> BTreeMap<String, &HirCleanupFooter> {
    cleanup_footers
        .iter()
        .filter(|rollup| !rollup.subject.is_empty())
        .map(|rollup| (rollup.subject.clone(), rollup))
        .collect()
}

fn effective_cleanup_cleanup_footers<'a>(
    cleanup_footers: &'a [HirCleanupFooter],
    inherited_cleanup_footers: &'a [HirCleanupFooter],
) -> &'a [HirCleanupFooter] {
    if cleanup_footers.is_empty() {
        inherited_cleanup_footers
    } else {
        cleanup_footers
    }
}

fn hir_type_is_move_owned(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    ty: &HirType,
) -> bool {
    match &ty.kind {
        HirTypeKind::Ref { .. } => false,
        HirTypeKind::Path(path) if path.segments.len() == 1 => matches!(
            path.segments[0].as_str(),
            "Str"
                | "List"
                | "Array"
                | "Map"
                | "Arena"
                | "FrameArena"
                | "PoolArena"
                | "Task"
                | "Thread"
                | "Channel"
                | "Mutex"
        ),
        HirTypeKind::Path(path) | HirTypeKind::Apply { base: path, .. } => {
            let Some(symbol_ref) = lookup_symbol_path(workspace, resolved_module, &path.segments)
            else {
                return false;
            };
            match symbol_ref.symbol.kind {
                HirSymbolKind::OpaqueType => matches!(
                    symbol_ref
                        .symbol
                        .opaque_policy
                        .map(|policy| policy.ownership),
                    Some(arcana_hir::HirOpaqueOwnershipPolicy::Move)
                ),
                HirSymbolKind::Record | HirSymbolKind::Object | HirSymbolKind::Enum => true,
                _ => false,
            }
        }
        HirTypeKind::Tuple(_) => true,
        _ => false,
    }
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
                found.insert(resolved_symbol_path(symbol_ref));
            }
        }
    }
    match found.into_iter().collect::<Vec<_>>().as_slice() {
        [] => Err("no `cleanup_contract` lang item is available".to_string()),
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

fn resolve_default_cleanup_routine_for_type(
    scope: &ResolvedRenderScope<'_>,
    ty: &HirType,
) -> Result<String, String> {
    let contract_path =
        resolve_cleanup_contract_trait_path(scope.workspace, scope.resolved_module)?;
    let candidates = lookup_method_candidates_for_hir_type(
        scope.workspace,
        scope.resolved_module,
        ty,
        "cleanup",
    )
    .into_iter()
    .filter(|candidate| candidate.trait_path.as_ref() == Some(&contract_path))
    .collect::<Vec<_>>();
    match candidates.as_slice() {
        [] => Err(format!(
            "type `{}` does not have a concrete `Cleanup` impl for cleanup footer lowering",
            ty.render()
        )),
        [candidate] => Ok(candidate.routine_key.clone()),
        _ => Err(format!(
            "cleanup footer lowering is ambiguous for `{}`; multiple concrete `Cleanup.cleanup` impls are visible",
            ty.render()
        )),
    }
}

fn lower_cleanup_entry_for_binding(
    scope: &ResolvedRenderScope<'_>,
    binding: &CleanupBindingCandidate,
    override_rollup: Option<&HirCleanupFooter>,
) -> Result<ExecCleanupFooter, String> {
    if let Some(rollup) = override_rollup
        && !rollup.handler_path.is_empty()
    {
        let _ = resolve_default_cleanup_routine_for_type(scope, &binding.ty)?;
        let (handler_path, resolved_routine) =
            lookup_symbol_path(scope.workspace, scope.resolved_module, &rollup.handler_path)
                .map(|symbol_ref| {
                    (
                        resolved_symbol_path(symbol_ref),
                        Some(routine_key_for_symbol(
                            symbol_ref.module_id,
                            symbol_ref.symbol_index,
                        )),
                    )
                })
                .unwrap_or_else(|| (rollup.handler_path.clone(), None));
        return Ok(ExecCleanupFooter {
            kind: "cleanup".to_string(),
            binding_id: binding.binding_id,
            subject: binding.name.clone(),
            handler_path,
            resolved_routine,
        });
    }
    Ok(ExecCleanupFooter {
        kind: "cleanup".to_string(),
        binding_id: binding.binding_id,
        subject: binding.name.clone(),
        handler_path: vec!["cleanup".to_string()],
        resolved_routine: Some(resolve_default_cleanup_routine_for_type(
            scope,
            &binding.ty,
        )?),
    })
}

fn push_cleanup_binding_candidate(
    bindings: &mut Vec<CleanupBindingCandidate>,
    name: String,
    binding_id: u64,
    ty: HirType,
) {
    if bindings
        .iter()
        .any(|existing| existing.binding_id == binding_id)
    {
        return;
    }
    bindings.push(CleanupBindingCandidate {
        name,
        binding_id,
        ty,
    });
}

fn collect_lowered_cleanup_bindings(
    scope: &ResolvedRenderScope<'_>,
    statement: &ExecStmt,
    bindings: &mut Vec<CleanupBindingCandidate>,
) {
    match statement {
        ExecStmt::Let {
            binding_id, name, ..
        } => {
            if *binding_id == 0 {
                return;
            }
            if let Some(ty) = scope.value_scope.type_of(name).cloned() {
                push_cleanup_binding_candidate(bindings, name.clone(), *binding_id, ty);
            }
        }
        ExecStmt::ActivateOwner {
            object_binding_ids, ..
        } => {
            for object_binding in object_binding_ids {
                if object_binding.binding_id == 0 {
                    continue;
                }
                if let Some(ty) = scope.value_scope.type_of(&object_binding.name).cloned() {
                    push_cleanup_binding_candidate(
                        bindings,
                        object_binding.name.clone(),
                        object_binding.binding_id,
                        ty,
                    );
                }
            }
        }
        ExecStmt::Recycle { lines, .. } => {
            for line in lines {
                let ExecRecycleLineKind::Let { name, .. } = &line.kind else {
                    continue;
                };
                let Some(binding_id) = scope.value_scope.binding_id_of(name) else {
                    continue;
                };
                let Some(ty) = scope.value_scope.type_of(name).cloned() else {
                    continue;
                };
                push_cleanup_binding_candidate(bindings, name.clone(), binding_id, ty);
            }
        }
        ExecStmt::Bind { lines, .. } => {
            for line in lines {
                let ExecBindLineKind::Let { name, .. } = &line.kind else {
                    continue;
                };
                let Some(binding_id) = scope.value_scope.binding_id_of(name) else {
                    continue;
                };
                let Some(ty) = scope.value_scope.type_of(name).cloned() else {
                    continue;
                };
                push_cleanup_binding_candidate(bindings, name.clone(), binding_id, ty);
            }
        }
        ExecStmt::Construct(region) => {
            let Some(ExecConstructDestination::Deliver { name }) = &region.destination else {
                return;
            };
            let Some(binding_id) = scope.value_scope.binding_id_of(name) else {
                return;
            };
            let Some(ty) = scope.value_scope.type_of(name).cloned() else {
                return;
            };
            push_cleanup_binding_candidate(bindings, name.clone(), binding_id, ty);
        }
        _ => {}
    }
}

fn lower_resolved_cleanup_cleanup_footers_for_bindings(
    scope: &ResolvedRenderScope<'_>,
    cleanup_footers: &[HirCleanupFooter],
    bindings: Vec<CleanupBindingCandidate>,
) -> Result<Vec<ExecCleanupFooter>, String> {
    if cleanup_footers.is_empty() {
        return Ok(Vec::new());
    }
    let has_bare = has_bare_cleanup_rollup(cleanup_footers);
    let targeted = targeted_cleanup_cleanup_footers(cleanup_footers);
    let mut bindings_by_name = BTreeMap::<String, Vec<&CleanupBindingCandidate>>::new();
    for binding in &bindings {
        bindings_by_name
            .entry(binding.name.clone())
            .or_default()
            .push(binding);
    }
    let mut explicit_binding_ids = BTreeMap::<String, u64>::new();
    for target in targeted.keys() {
        let Some(candidates) = bindings_by_name.get(target) else {
            return Err(format!(
                "cleanup footer target `{target}` is not cleanup-capable in the owning scope"
            ));
        };
        match candidates.as_slice() {
            [binding] => {
                explicit_binding_ids.insert(target.clone(), binding.binding_id);
            }
            _ => {
                return Err(format!(
                    "cleanup footer target `{target}` is ambiguous in the owning scope"
                ));
            }
        }
    }
    let mut lowered = Vec::new();
    for binding in bindings {
        if let Some(explicit_binding_id) = explicit_binding_ids.get(&binding.name)
            && *explicit_binding_id == binding.binding_id
        {
            lowered.push(lower_cleanup_entry_for_binding(
                scope,
                &binding,
                targeted.get(&binding.name).copied(),
            )?);
            continue;
        }
        let eligible = hir_type_is_move_owned(scope.workspace, scope.resolved_module, &binding.ty);
        if has_bare
            && eligible
            && resolve_default_cleanup_routine_for_type(scope, &binding.ty).is_ok()
        {
            lowered.push(lower_cleanup_entry_for_binding(scope, &binding, None)?);
        }
    }
    Ok(lowered)
}

fn lower_resolved_symbol_cleanup_footers(
    scope: &ResolvedRenderScope<'_>,
    symbol: &HirSymbol,
    immediate_bindings: Vec<CleanupBindingCandidate>,
) -> Result<Vec<ExecCleanupFooter>, String> {
    if symbol.cleanup_footers.is_empty() {
        return Ok(Vec::new());
    }
    let mut bindings = Vec::new();
    let targeted = targeted_cleanup_cleanup_footers(&symbol.cleanup_footers);
    for param in &symbol.params {
        let explicitly_targeted = targeted.contains_key(&param.name);
        if explicitly_targeted
            && matches!(
                param.mode,
                Some(arcana_hir::HirParamMode::Read | arcana_hir::HirParamMode::Edit)
            )
        {
            return Err(format!(
                "cleanup footer target `{}` must be an owning binding; `read`/`edit` params are ineligible",
                param.name
            ));
        }
        let is_eligible_param =
            !matches!(
                param.mode,
                Some(arcana_hir::HirParamMode::Read | arcana_hir::HirParamMode::Edit)
            ) && hir_type_is_move_owned(scope.workspace, scope.resolved_module, &param.ty);
        if (explicitly_targeted || is_eligible_param)
            && let Some(binding_id) = scope.value_scope.binding_id_of(&param.name)
        {
            push_cleanup_binding_candidate(
                &mut bindings,
                param.name.clone(),
                binding_id,
                param.ty.clone(),
            );
        }
    }
    bindings.extend(immediate_bindings);
    lower_resolved_cleanup_cleanup_footers_for_bindings(scope, &symbol.cleanup_footers, bindings)
}

fn lower_resolved_statement_cleanup_footers(
    scope: &ResolvedRenderScope<'_>,
    cleanup_footers: &[HirCleanupFooter],
    bindings: Vec<CleanupBindingCandidate>,
) -> Result<Vec<ExecCleanupFooter>, String> {
    if cleanup_footers.is_empty() {
        return Ok(Vec::new());
    }
    lower_resolved_cleanup_cleanup_footers_for_bindings(scope, cleanup_footers, bindings)
}

fn lower_availability_attachment_exec(
    attachment: &arcana_hir::HirAvailabilityAttachment,
) -> ExecAvailabilityAttachment {
    ExecAvailabilityAttachment {
        kind: ExecAvailabilityKind::Object,
        path: attachment.path.clone(),
        local_name: attachment.path.last().cloned().unwrap_or_default(),
    }
}

fn canonical_symbol_path(module_id: &str, symbol_name: &str) -> Vec<String> {
    let mut path = module_id.split('.').map(str::to_string).collect::<Vec<_>>();
    path.push(symbol_name.to_string());
    path
}

fn lower_availability_attachment_exec_resolved(
    attachment: &arcana_hir::HirAvailabilityAttachment,
    scope: &ResolvedRenderScope<'_>,
) -> Result<ExecAvailabilityAttachment, String> {
    let resolved = lookup_symbol_path(scope.workspace, scope.resolved_module, &attachment.path)
        .ok_or_else(|| {
            format!(
                "unresolved availability attachment `{}`",
                attachment.path.join(".")
            )
        })?;
    let kind = match resolved.symbol.kind {
        HirSymbolKind::Owner => ExecAvailabilityKind::Owner,
        HirSymbolKind::Object => ExecAvailabilityKind::Object,
        other => {
            return Err(format!(
                "availability attachment `{}` must resolve to owner or object, found `{}`",
                attachment.path.join("."),
                other.as_str()
            ));
        }
    };
    Ok(ExecAvailabilityAttachment {
        kind,
        path: canonical_symbol_path(resolved.module_id, &resolved.symbol.name),
        local_name: resolved.symbol.name.clone(),
    })
}

fn lower_chain_step_exec(step: &HirChainStep) -> ExecChainStep {
    ExecChainStep {
        incoming: step.incoming.map(lower_chain_connector),
        stage: lower_exec_expr(&step.stage),
        bind_args: step.bind_args.iter().map(lower_exec_expr).collect(),
        text: step.text.clone(),
    }
}

fn lower_chain_step_exec_resolved(
    step: &HirChainStep,
    scope: &ResolvedRenderScope<'_>,
) -> ExecChainStep {
    ExecChainStep {
        incoming: step.incoming.map(lower_chain_connector),
        stage: lower_exec_expr_resolved(&step.stage, scope),
        bind_args: step
            .bind_args
            .iter()
            .map(|expr| lower_exec_expr_resolved(expr, scope))
            .collect(),
        text: step.text.clone(),
    }
}

fn lower_assign_target_exec(target: &HirAssignTarget) -> ExecAssignTarget {
    match target {
        HirAssignTarget::Name { text } => ExecAssignTarget::Name(text.clone()),
        HirAssignTarget::Deref { expr } => lower_assign_target_exec_from_expr(expr)
            .expect("deref assignment target should lower from an assignable expression"),
        HirAssignTarget::MemberAccess { target, member } => ExecAssignTarget::Member {
            target: Box::new(lower_assign_target_exec(target)),
            member: member.clone(),
        },
        HirAssignTarget::Index { target, index } => ExecAssignTarget::Index {
            target: Box::new(lower_assign_target_exec(target)),
            index: lower_exec_expr(index),
        },
    }
}

fn lower_assign_target_exec_resolved(
    target: &HirAssignTarget,
    scope: &ResolvedRenderScope<'_>,
) -> ExecAssignTarget {
    match target {
        HirAssignTarget::Name { text } => ExecAssignTarget::Name(text.clone()),
        HirAssignTarget::Deref { expr } => lower_assign_target_exec_resolved_from_expr(expr, scope)
            .expect("resolved deref assignment target should lower from an assignable expression"),
        HirAssignTarget::MemberAccess { target, member } => ExecAssignTarget::Member {
            target: Box::new(lower_assign_target_exec_resolved(target, scope)),
            member: member.clone(),
        },
        HirAssignTarget::Index { target, index } => ExecAssignTarget::Index {
            target: Box::new(lower_assign_target_exec_resolved(target, scope)),
            index: lower_exec_expr_resolved(index, scope),
        },
    }
}

fn lower_assign_target_exec_from_expr(expr: &HirExpr) -> Option<ExecAssignTarget> {
    match expr {
        HirExpr::Path { segments } if segments.len() == 1 => {
            Some(ExecAssignTarget::Name(segments[0].clone()))
        }
        HirExpr::MemberAccess { expr, member } => Some(ExecAssignTarget::Member {
            target: Box::new(lower_assign_target_exec_from_expr(expr)?),
            member: member.clone(),
        }),
        HirExpr::Index { expr, index } => Some(ExecAssignTarget::Index {
            target: Box::new(lower_assign_target_exec_from_expr(expr)?),
            index: lower_exec_expr(index),
        }),
        HirExpr::GenericApply { expr, .. } => lower_assign_target_exec_from_expr(expr),
        HirExpr::Unary {
            op:
                HirUnaryOp::CapabilityRead
                | HirUnaryOp::CapabilityEdit
                | HirUnaryOp::CapabilityTake
                | HirUnaryOp::CapabilityHold
                | HirUnaryOp::Deref,
            expr,
        } => lower_assign_target_exec_from_expr(expr),
        _ => None,
    }
}

fn lower_assign_target_exec_resolved_from_expr(
    expr: &HirExpr,
    scope: &ResolvedRenderScope<'_>,
) -> Option<ExecAssignTarget> {
    match expr {
        HirExpr::Path { segments } if segments.len() == 1 => {
            Some(ExecAssignTarget::Name(segments[0].clone()))
        }
        HirExpr::MemberAccess { expr, member } => Some(ExecAssignTarget::Member {
            target: Box::new(lower_assign_target_exec_resolved_from_expr(expr, scope)?),
            member: member.clone(),
        }),
        HirExpr::Index { expr, index } => Some(ExecAssignTarget::Index {
            target: Box::new(lower_assign_target_exec_resolved_from_expr(expr, scope)?),
            index: lower_exec_expr_resolved(index, scope),
        }),
        HirExpr::GenericApply { expr, .. } => {
            lower_assign_target_exec_resolved_from_expr(expr, scope)
        }
        HirExpr::Unary {
            op:
                HirUnaryOp::CapabilityRead
                | HirUnaryOp::CapabilityEdit
                | HirUnaryOp::CapabilityTake
                | HirUnaryOp::CapabilityHold
                | HirUnaryOp::Deref,
            expr,
        } => lower_assign_target_exec_resolved_from_expr(expr, scope),
        _ => None,
    }
}

fn lower_headed_modifier_exec(modifier: &arcana_hir::HirHeadedModifier) -> ExecHeadedModifier {
    ExecHeadedModifier {
        kind: match &modifier.kind {
            arcana_hir::HirHeadedModifierKind::Keyword(keyword) => keyword.as_str().to_string(),
            arcana_hir::HirHeadedModifierKind::Name(name) => name.clone(),
        },
        payload: modifier.payload.as_ref().map(lower_exec_expr),
    }
}

fn lower_module_memory_spec_exec(spec: &arcana_hir::HirMemorySpecDecl) -> ExecMemorySpecDecl {
    ExecMemorySpecDecl {
        family: spec.family.as_str().to_string(),
        name: spec.name.clone(),
        default_modifier: spec
            .default_modifier
            .as_ref()
            .map(lower_headed_modifier_exec),
        details: spec
            .details
            .iter()
            .map(|detail| ExecMemoryDetailLine {
                key: detail.key.as_str().to_string(),
                value: lower_exec_expr(&detail.value),
                modifier: detail.modifier.as_ref().map(lower_headed_modifier_exec),
            })
            .collect(),
    }
}

fn lower_headed_modifier_exec_resolved(
    modifier: &arcana_hir::HirHeadedModifier,
    scope: &ResolvedRenderScope<'_>,
) -> ExecHeadedModifier {
    ExecHeadedModifier {
        kind: match &modifier.kind {
            arcana_hir::HirHeadedModifierKind::Keyword(keyword) => keyword.as_str().to_string(),
            arcana_hir::HirHeadedModifierKind::Name(name) => name.clone(),
        },
        payload: modifier
            .payload
            .as_ref()
            .map(|payload| lower_exec_expr_resolved(payload, scope)),
    }
}

fn lower_construct_region_exec(region: &arcana_hir::HirConstructRegion) -> ExecConstructRegion {
    ExecConstructRegion {
        completion: region.completion.as_str().to_string(),
        target: Box::new(lower_exec_expr(&region.target)),
        destination: region
            .destination
            .as_ref()
            .map(|destination| match destination {
                arcana_hir::HirConstructDestination::Deliver { name } => {
                    ExecConstructDestination::Deliver { name: name.clone() }
                }
                arcana_hir::HirConstructDestination::Place { target } => {
                    ExecConstructDestination::Place {
                        target: lower_assign_target_exec(target),
                    }
                }
            }),
        default_modifier: region
            .default_modifier
            .as_ref()
            .map(lower_headed_modifier_exec),
        lines: region
            .lines
            .iter()
            .map(|line| ExecConstructLine {
                name: line.name.clone(),
                value: lower_exec_expr(&line.value),
                mode: ExecConstructContributionMode::Direct,
                modifier: line.modifier.as_ref().map(lower_headed_modifier_exec),
            })
            .collect(),
    }
}

fn lower_record_region_exec(region: &arcana_hir::HirRecordRegion) -> ExecRecordRegion {
    ExecRecordRegion {
        kind: region.kind.as_str().to_string(),
        completion: region.completion.as_str().to_string(),
        target: Box::new(lower_exec_expr(&region.target)),
        base: region
            .base
            .as_ref()
            .map(|base| Box::new(lower_exec_expr(base))),
        destination: region
            .destination
            .as_ref()
            .map(|destination| match destination {
                arcana_hir::HirConstructDestination::Deliver { name } => {
                    ExecConstructDestination::Deliver { name: name.clone() }
                }
                arcana_hir::HirConstructDestination::Place { target } => {
                    ExecConstructDestination::Place {
                        target: lower_assign_target_exec(target),
                    }
                }
            }),
        default_modifier: region
            .default_modifier
            .as_ref()
            .map(lower_headed_modifier_exec),
        lines: region
            .lines
            .iter()
            .map(|line| ExecConstructLine {
                name: line.name.clone(),
                value: lower_exec_expr(&line.value),
                mode: ExecConstructContributionMode::Direct,
                modifier: line.modifier.as_ref().map(lower_headed_modifier_exec),
            })
            .collect(),
        copied_fields: Vec::new(),
    }
}

fn lower_array_region_exec(region: &arcana_hir::HirArrayRegion) -> ExecArrayRegion {
    ExecArrayRegion {
        completion: region.completion.as_str().to_string(),
        target: Box::new(lower_exec_expr(&region.target)),
        base: region
            .base
            .as_ref()
            .map(|base| Box::new(lower_exec_expr(base))),
        destination: region
            .destination
            .as_ref()
            .map(|destination| match destination {
                arcana_hir::HirConstructDestination::Deliver { name } => {
                    ExecConstructDestination::Deliver { name: name.clone() }
                }
                arcana_hir::HirConstructDestination::Place { target } => {
                    ExecConstructDestination::Place {
                        target: lower_assign_target_exec(target),
                    }
                }
            }),
        default_modifier: region
            .default_modifier
            .as_ref()
            .map(lower_headed_modifier_exec),
        lines: region
            .lines
            .iter()
            .map(|line| ExecArrayLine {
                index: line.index,
                value: lower_exec_expr(&line.value),
                modifier: line.modifier.as_ref().map(lower_headed_modifier_exec),
            })
            .collect(),
    }
}

fn lower_construct_region_exec_resolved(
    region: &arcana_hir::HirConstructRegion,
    scope: &ResolvedRenderScope<'_>,
) -> ExecConstructRegion {
    ExecConstructRegion {
        completion: region.completion.as_str().to_string(),
        target: Box::new(lower_exec_expr_resolved(&region.target, scope)),
        destination: region
            .destination
            .as_ref()
            .map(|destination| match destination {
                arcana_hir::HirConstructDestination::Deliver { name } => {
                    ExecConstructDestination::Deliver { name: name.clone() }
                }
                arcana_hir::HirConstructDestination::Place { target } => {
                    ExecConstructDestination::Place {
                        target: lower_assign_target_exec_resolved(target, scope),
                    }
                }
            }),
        default_modifier: region
            .default_modifier
            .as_ref()
            .map(|modifier| lower_headed_modifier_exec_resolved(modifier, scope)),
        lines: region
            .lines
            .iter()
            .map(|line| ExecConstructLine {
                name: line.name.clone(),
                value: lower_exec_expr_resolved(&line.value, scope),
                mode: infer_construct_contribution_mode(scope, region, line),
                modifier: line
                    .modifier
                    .as_ref()
                    .map(|modifier| lower_headed_modifier_exec_resolved(modifier, scope)),
            })
            .collect(),
    }
}

fn lower_record_region_exec_resolved(
    region: &arcana_hir::HirRecordRegion,
    scope: &ResolvedRenderScope<'_>,
) -> ExecRecordRegion {
    ExecRecordRegion {
        kind: region.kind.as_str().to_string(),
        completion: region.completion.as_str().to_string(),
        target: Box::new(lower_exec_expr_resolved(&region.target, scope)),
        base: region
            .base
            .as_ref()
            .map(|base| Box::new(lower_exec_expr_resolved(base, scope))),
        destination: region
            .destination
            .as_ref()
            .map(|destination| match destination {
                arcana_hir::HirConstructDestination::Deliver { name } => {
                    ExecConstructDestination::Deliver { name: name.clone() }
                }
                arcana_hir::HirConstructDestination::Place { target } => {
                    ExecConstructDestination::Place {
                        target: lower_assign_target_exec_resolved(target, scope),
                    }
                }
            }),
        default_modifier: region
            .default_modifier
            .as_ref()
            .map(|modifier| lower_headed_modifier_exec_resolved(modifier, scope)),
        lines: region
            .lines
            .iter()
            .map(|line| ExecConstructLine {
                name: line.name.clone(),
                value: lower_exec_expr_resolved(&line.value, scope),
                mode: infer_record_contribution_mode(scope, region, line),
                modifier: line
                    .modifier
                    .as_ref()
                    .map(|modifier| lower_headed_modifier_exec_resolved(modifier, scope)),
            })
            .collect(),
        copied_fields: collect_record_copied_fields(scope, region),
    }
}

fn lower_array_region_exec_resolved(
    region: &arcana_hir::HirArrayRegion,
    scope: &ResolvedRenderScope<'_>,
) -> ExecArrayRegion {
    ExecArrayRegion {
        completion: region.completion.as_str().to_string(),
        target: Box::new(lower_exec_expr_resolved(&region.target, scope)),
        base: region
            .base
            .as_ref()
            .map(|base| Box::new(lower_exec_expr_resolved(base, scope))),
        destination: region
            .destination
            .as_ref()
            .map(|destination| match destination {
                arcana_hir::HirConstructDestination::Deliver { name } => {
                    ExecConstructDestination::Deliver { name: name.clone() }
                }
                arcana_hir::HirConstructDestination::Place { target } => {
                    ExecConstructDestination::Place {
                        target: lower_assign_target_exec_resolved(target, scope),
                    }
                }
            }),
        default_modifier: region
            .default_modifier
            .as_ref()
            .map(|modifier| lower_headed_modifier_exec_resolved(modifier, scope)),
        lines: region
            .lines
            .iter()
            .map(|line| ExecArrayLine {
                index: line.index,
                value: lower_exec_expr_resolved(&line.value, scope),
                modifier: line
                    .modifier
                    .as_ref()
                    .map(|modifier| lower_headed_modifier_exec_resolved(modifier, scope)),
            })
            .collect(),
    }
}

fn lower_exec_expr(expr: &HirExpr) -> ExecExpr {
    match expr {
        HirExpr::Path { segments } => ExecExpr::Path(segments.clone()),
        HirExpr::BoolLiteral { value } => ExecExpr::Bool(*value),
        HirExpr::IntLiteral { text } => ExecExpr::Int(text.parse().unwrap_or_default()),
        HirExpr::FloatLiteral { text, kind } => ExecExpr::Float {
            text: text.clone(),
            kind: match kind {
                arcana_hir::HirFloatLiteralKind::F32 => ExecFloatKind::F32,
                arcana_hir::HirFloatLiteralKind::F64 => ExecFloatKind::F64,
            },
        },
        HirExpr::StrLiteral { text } => {
            ExecExpr::Str(decode_source_string_literal(text).unwrap_or_else(|_| text.clone()))
        }
        HirExpr::Pair { left, right } => ExecExpr::Pair {
            left: Box::new(lower_exec_expr(left)),
            right: Box::new(lower_exec_expr(right)),
        },
        HirExpr::CollectionLiteral { items } => ExecExpr::Collection {
            items: items.iter().map(lower_exec_expr).collect(),
        },
        HirExpr::Match { subject, arms } => ExecExpr::Match {
            subject: Box::new(lower_exec_expr(subject)),
            arms: arms
                .iter()
                .map(|arm| ExecMatchArm {
                    patterns: arm.patterns.iter().map(lower_match_pattern_exec).collect(),
                    value: lower_exec_expr(&arm.value),
                })
                .collect(),
        },
        HirExpr::ConstructRegion(region) => {
            ExecExpr::ConstructRegion(Box::new(lower_construct_region_exec(region)))
        }
        HirExpr::RecordRegion(region) => {
            ExecExpr::RecordRegion(Box::new(lower_record_region_exec(region)))
        }
        HirExpr::ArrayRegion(region) => {
            ExecExpr::ArrayRegion(Box::new(lower_array_region_exec(region)))
        }
        HirExpr::Chain {
            style,
            introducer,
            steps,
        } => ExecExpr::Chain {
            style: style.clone(),
            introducer: lower_chain_introducer(*introducer),
            steps: steps.iter().map(lower_chain_step_exec).collect(),
        },
        HirExpr::MemoryPhrase {
            family,
            arena,
            init_args,
            constructor,
            attached,
        } => ExecExpr::MemoryPhrase {
            family: family.clone(),
            arena: Box::new(lower_exec_expr(arena)),
            init_args: init_args.iter().map(lower_phrase_arg_exec).collect(),
            constructor: Box::new(lower_exec_expr(constructor)),
            attached: attached.iter().map(lower_header_attachment_exec).collect(),
        },
        HirExpr::GenericApply { expr, type_args } => ExecExpr::Generic {
            expr: Box::new(lower_exec_expr(expr)),
            type_args: type_args.iter().map(arcana_hir::HirType::render).collect(),
        },
        HirExpr::QualifiedPhrase {
            subject,
            args,
            qualifier_kind,
            qualifier,
            qualifier_type_args,
            attached,
        } => ExecExpr::Phrase {
            subject: Box::new(lower_exec_expr(subject)),
            args: args.iter().map(lower_phrase_arg_exec).collect(),
            qualifier_kind: lower_phrase_qualifier_kind(*qualifier_kind),
            qualifier: qualifier.clone(),
            qualifier_type_args: qualifier_type_args
                .iter()
                .map(arcana_hir::HirType::render)
                .collect(),
            resolved_callable: None,
            resolved_routine: None,
            dynamic_dispatch: None,
            attached: attached.iter().map(lower_header_attachment_exec).collect(),
        },
        HirExpr::Await { expr } => ExecExpr::Await {
            expr: Box::new(lower_exec_expr(expr)),
        },
        HirExpr::Unary { op, expr } => ExecExpr::Unary {
            op: lower_unary_op(*op),
            expr: Box::new(lower_exec_expr(expr)),
        },
        HirExpr::Binary { left, op, right } => ExecExpr::Binary {
            left: Box::new(lower_exec_expr(left)),
            op: lower_binary_op(*op),
            right: Box::new(lower_exec_expr(right)),
        },
        HirExpr::MemberAccess { expr, member } => ExecExpr::Member {
            expr: Box::new(lower_exec_expr(expr)),
            member: member.clone(),
        },
        HirExpr::Index { expr, index } => ExecExpr::Index {
            expr: Box::new(lower_exec_expr(expr)),
            index: Box::new(lower_exec_expr(index)),
        },
        HirExpr::Slice {
            expr,
            start,
            end,
            inclusive_end,
        } => ExecExpr::Slice {
            expr: Box::new(lower_exec_expr(expr)),
            start: start.as_ref().map(|expr| Box::new(lower_exec_expr(expr))),
            end: end.as_ref().map(|expr| Box::new(lower_exec_expr(expr))),
            inclusive_end: *inclusive_end,
        },
        HirExpr::Range {
            start,
            end,
            inclusive_end,
        } => ExecExpr::Range {
            start: start.as_ref().map(|expr| Box::new(lower_exec_expr(expr))),
            end: end.as_ref().map(|expr| Box::new(lower_exec_expr(expr))),
            inclusive_end: *inclusive_end,
        },
    }
}

fn lower_exec_expr_resolved(expr: &HirExpr, scope: &ResolvedRenderScope<'_>) -> ExecExpr {
    match expr {
        HirExpr::Path { segments } => ExecExpr::Path(segments.clone()),
        HirExpr::BoolLiteral { value } => ExecExpr::Bool(*value),
        HirExpr::IntLiteral { text } => ExecExpr::Int(text.parse().unwrap_or_default()),
        HirExpr::FloatLiteral { text, kind } => ExecExpr::Float {
            text: text.clone(),
            kind: match kind {
                arcana_hir::HirFloatLiteralKind::F32 => ExecFloatKind::F32,
                arcana_hir::HirFloatLiteralKind::F64 => ExecFloatKind::F64,
            },
        },
        HirExpr::StrLiteral { text } => {
            ExecExpr::Str(decode_source_string_literal(text).unwrap_or_else(|_| text.clone()))
        }
        HirExpr::Pair { left, right } => ExecExpr::Pair {
            left: Box::new(lower_exec_expr_resolved(left, scope)),
            right: Box::new(lower_exec_expr_resolved(right, scope)),
        },
        HirExpr::CollectionLiteral { items } => ExecExpr::Collection {
            items: items
                .iter()
                .map(|item| lower_exec_expr_resolved(item, scope))
                .collect(),
        },
        HirExpr::Match { subject, arms } => ExecExpr::Match {
            subject: Box::new(lower_exec_expr_resolved(subject, scope)),
            arms: arms
                .iter()
                .map(|arm| ExecMatchArm {
                    patterns: arm
                        .patterns
                        .iter()
                        .map(|pattern| {
                            lower_subject_match_pattern_exec_resolved(pattern, subject, scope)
                        })
                        .collect(),
                    value: lower_exec_expr_resolved(&arm.value, scope),
                })
                .collect(),
        },
        HirExpr::ConstructRegion(region) => ExecExpr::ConstructRegion(Box::new(
            lower_construct_region_exec_resolved(region, scope),
        )),
        HirExpr::RecordRegion(region) => {
            ExecExpr::RecordRegion(Box::new(lower_record_region_exec_resolved(region, scope)))
        }
        HirExpr::ArrayRegion(region) => {
            ExecExpr::ArrayRegion(Box::new(lower_array_region_exec_resolved(region, scope)))
        }
        HirExpr::Chain {
            style,
            introducer,
            steps,
        } => ExecExpr::Chain {
            style: style.clone(),
            introducer: lower_chain_introducer(*introducer),
            steps: steps
                .iter()
                .map(|step| lower_chain_step_exec_resolved(step, scope))
                .collect(),
        },
        HirExpr::MemoryPhrase {
            family,
            arena,
            init_args,
            constructor,
            attached,
        } => ExecExpr::MemoryPhrase {
            family: family.clone(),
            arena: Box::new(lower_exec_expr_resolved(arena, scope)),
            init_args: init_args
                .iter()
                .map(|arg| lower_phrase_arg_exec_resolved(arg, scope))
                .collect(),
            constructor: Box::new(lower_exec_expr_resolved(constructor, scope)),
            attached: attached
                .iter()
                .map(|attachment| lower_header_attachment_exec_resolved(attachment, scope))
                .collect(),
        },
        HirExpr::GenericApply { expr, type_args } => ExecExpr::Generic {
            expr: Box::new(lower_exec_expr_resolved(expr, scope)),
            type_args: type_args.iter().map(arcana_hir::HirType::render).collect(),
        },
        HirExpr::QualifiedPhrase {
            subject,
            args,
            qualifier_kind,
            qualifier,
            qualifier_type_args,
            attached,
        } => {
            let hir_qualifier_kind = *qualifier_kind;
            let qualifier_kind = lower_phrase_qualifier_kind(hir_qualifier_kind);
            let resolved = match qualifier_kind {
                ExecPhraseQualifierKind::Call
                | ExecPhraseQualifierKind::Weave
                | ExecPhraseQualifierKind::Split
                | ExecPhraseQualifierKind::NamedPath => resolve_qualified_phrase_target_path(
                    scope,
                    subject,
                    hir_qualifier_kind,
                    qualifier,
                ),
                ExecPhraseQualifierKind::BareMethod => {
                    resolve_bare_method_target(scope, subject, qualifier)
                }
                _ => None,
            };
            ExecExpr::Phrase {
                subject: Box::new(lower_exec_expr_resolved(subject, scope)),
                args: args
                    .iter()
                    .map(|arg| lower_phrase_arg_exec_resolved(arg, scope))
                    .collect(),
                qualifier_kind,
                qualifier: qualifier.clone(),
                qualifier_type_args: qualifier_type_args
                    .iter()
                    .map(arcana_hir::HirType::render)
                    .collect(),
                resolved_callable: resolved.as_ref().map(|target| target.path.clone()),
                resolved_routine: resolved
                    .as_ref()
                    .and_then(|target| target.routine_key.clone()),
                dynamic_dispatch: resolved.and_then(|target| target.dynamic_dispatch),
                attached: attached
                    .iter()
                    .map(|attachment| lower_header_attachment_exec_resolved(attachment, scope))
                    .collect(),
            }
        }
        HirExpr::Await { expr } => ExecExpr::Await {
            expr: Box::new(lower_exec_expr_resolved(expr, scope)),
        },
        HirExpr::Unary { op, expr } => ExecExpr::Unary {
            op: lower_unary_op(*op),
            expr: Box::new(lower_exec_expr_resolved(expr, scope)),
        },
        HirExpr::Binary { left, op, right } => ExecExpr::Binary {
            left: Box::new(lower_exec_expr_resolved(left, scope)),
            op: lower_binary_op(*op),
            right: Box::new(lower_exec_expr_resolved(right, scope)),
        },
        HirExpr::MemberAccess { expr, member } => ExecExpr::Member {
            expr: Box::new(lower_exec_expr_resolved(expr, scope)),
            member: member.clone(),
        },
        HirExpr::Index { expr, index } => ExecExpr::Index {
            expr: Box::new(lower_exec_expr_resolved(expr, scope)),
            index: Box::new(lower_exec_expr_resolved(index, scope)),
        },
        HirExpr::Slice {
            expr,
            start,
            end,
            inclusive_end,
        } => ExecExpr::Slice {
            expr: Box::new(lower_exec_expr_resolved(expr, scope)),
            start: start
                .as_ref()
                .map(|expr| Box::new(lower_exec_expr_resolved(expr, scope))),
            end: end
                .as_ref()
                .map(|expr| Box::new(lower_exec_expr_resolved(expr, scope))),
            inclusive_end: *inclusive_end,
        },
        HirExpr::Range {
            start,
            end,
            inclusive_end,
        } => ExecExpr::Range {
            start: start
                .as_ref()
                .map(|expr| Box::new(lower_exec_expr_resolved(expr, scope))),
            end: end
                .as_ref()
                .map(|expr| Box::new(lower_exec_expr_resolved(expr, scope))),
            inclusive_end: *inclusive_end,
        },
    }
}

fn lower_exec_stmt_block(statements: &[HirStatement]) -> Vec<ExecStmt> {
    statements
        .iter()
        .flat_map(lower_exec_stmt_sequence)
        .collect()
}

fn lower_exec_stmt_sequence(statement: &HirStatement) -> Vec<ExecStmt> {
    match &statement.kind {
        HirStatementKind::Let {
            mutable,
            name,
            value,
        } => {
            let lowered_value = lower_exec_expr(value);
            let Some(pattern) = parse_binding_pattern(name) else {
                return vec![ExecStmt::Let {
                    binding_id: 0,
                    mutable: *mutable,
                    name: name.clone(),
                    value: lowered_value,
                }];
            };
            if !matches!(pattern, BindingPattern::Pair(_, _)) {
                return vec![ExecStmt::Let {
                    binding_id: 0,
                    mutable: *mutable,
                    name: name.clone(),
                    value: lowered_value,
                }];
            }
            let temp_name = "__arcana_tuple_let".to_string();
            let base_expr = ExecExpr::Path(vec![temp_name.clone()]);
            let mut lowered = vec![ExecStmt::Let {
                binding_id: 0,
                mutable: false,
                name: temp_name,
                value: lowered_value,
            }];
            let mut destructured = Vec::new();
            collect_binding_pattern_exec_lets(
                &pattern,
                &base_expr,
                &mut Vec::new(),
                &mut destructured,
                *mutable,
            );
            lowered.extend(destructured);
            lowered
        }
        HirStatementKind::Expr { expr } => vec![ExecStmt::Expr {
            expr: lower_exec_expr(expr),
            cleanup_footers: statement.cleanup_footers.iter().map(lower_rollup).collect(),
        }],
        HirStatementKind::Return { value } => vec![match value.as_ref() {
            Some(value) => ExecStmt::ReturnValue {
                value: lower_exec_expr(value),
            },
            None => ExecStmt::ReturnVoid,
        }],
        HirStatementKind::If {
            condition,
            then_branch,
            else_branch,
        } => vec![ExecStmt::If {
            condition: lower_exec_expr(condition),
            then_branch: lower_exec_stmt_block(then_branch),
            else_branch: else_branch
                .as_ref()
                .map(|branch| lower_exec_stmt_block(branch))
                .unwrap_or_default(),
            availability: statement
                .availability
                .iter()
                .map(lower_availability_attachment_exec)
                .collect(),
            cleanup_footers: statement.cleanup_footers.iter().map(lower_rollup).collect(),
        }],
        HirStatementKind::While { condition, body } => vec![ExecStmt::While {
            condition: lower_exec_expr(condition),
            body: lower_exec_stmt_block(body),
            availability: statement
                .availability
                .iter()
                .map(lower_availability_attachment_exec)
                .collect(),
            cleanup_footers: statement.cleanup_footers.iter().map(lower_rollup).collect(),
        }],
        HirStatementKind::For {
            binding,
            iterable,
            body,
        } => {
            let lowered_iterable = lower_exec_expr(iterable);
            let Some(pattern) = parse_binding_pattern(binding) else {
                return vec![ExecStmt::For {
                    binding_id: 0,
                    binding: binding.clone(),
                    iterable: lowered_iterable,
                    body: lower_exec_stmt_block(body),
                    availability: statement
                        .availability
                        .iter()
                        .map(lower_availability_attachment_exec)
                        .collect(),
                    cleanup_footers: statement.cleanup_footers.iter().map(lower_rollup).collect(),
                }];
            };
            if !matches!(pattern, BindingPattern::Pair(_, _)) {
                return vec![ExecStmt::For {
                    binding_id: 0,
                    binding: binding.clone(),
                    iterable: lowered_iterable,
                    body: lower_exec_stmt_block(body),
                    availability: statement
                        .availability
                        .iter()
                        .map(lower_availability_attachment_exec)
                        .collect(),
                    cleanup_footers: statement.cleanup_footers.iter().map(lower_rollup).collect(),
                }];
            }
            let temp_name = "__arcana_tuple_for".to_string();
            let base_expr = ExecExpr::Path(vec![temp_name.clone()]);
            let mut lowered_body = Vec::new();
            collect_binding_pattern_exec_lets(
                &pattern,
                &base_expr,
                &mut Vec::new(),
                &mut lowered_body,
                false,
            );
            lowered_body.extend(lower_exec_stmt_block(body));
            vec![ExecStmt::For {
                binding_id: 0,
                binding: temp_name,
                iterable: lowered_iterable,
                body: lowered_body,
                availability: statement
                    .availability
                    .iter()
                    .map(lower_availability_attachment_exec)
                    .collect(),
                cleanup_footers: statement.cleanup_footers.iter().map(lower_rollup).collect(),
            }]
        }
        HirStatementKind::Defer { action } => vec![ExecStmt::Defer(match action {
            arcana_hir::HirDeferAction::Expr { expr } => {
                ExecDeferAction::Expr(lower_exec_expr(expr))
            }
            arcana_hir::HirDeferAction::Reclaim { expr } => {
                ExecDeferAction::Reclaim(lower_exec_expr(expr))
            }
        })],
        HirStatementKind::Reclaim { expr } => vec![ExecStmt::Reclaim(lower_exec_expr(expr))],
        HirStatementKind::Break => vec![ExecStmt::Break],
        HirStatementKind::Continue => vec![ExecStmt::Continue],
        HirStatementKind::Assign { target, op, value } => vec![ExecStmt::Assign {
            target: lower_assign_target_exec(target),
            op: lower_assign_op(*op),
            value: lower_exec_expr(value),
        }],
        HirStatementKind::Recycle {
            default_modifier,
            lines,
        } => vec![ExecStmt::Recycle {
            default_modifier: default_modifier.as_ref().map(lower_headed_modifier_exec),
            lines: lines
                .iter()
                .map(|line| ExecRecycleLine {
                    kind: match &line.kind {
                        arcana_hir::HirRecycleLineKind::Expr { gate } => {
                            ExecRecycleLineKind::Expr {
                                gate: lower_exec_expr(gate),
                            }
                        }
                        arcana_hir::HirRecycleLineKind::Let {
                            mutable,
                            name,
                            gate,
                        } => ExecRecycleLineKind::Let {
                            mutable: *mutable,
                            name: name.clone(),
                            gate: lower_exec_expr(gate),
                        },
                        arcana_hir::HirRecycleLineKind::Assign { name, gate } => {
                            ExecRecycleLineKind::Assign {
                                name: name.clone(),
                                gate: lower_exec_expr(gate),
                            }
                        }
                    },
                    modifier: line.modifier.as_ref().map(lower_headed_modifier_exec),
                })
                .collect(),
        }],
        HirStatementKind::Bind {
            default_modifier,
            lines,
        } => vec![ExecStmt::Bind {
            default_modifier: default_modifier.as_ref().map(lower_headed_modifier_exec),
            lines: lines
                .iter()
                .map(|line| ExecBindLine {
                    kind: match &line.kind {
                        arcana_hir::HirBindLineKind::Let {
                            mutable,
                            name,
                            gate,
                        } => ExecBindLineKind::Let {
                            mutable: *mutable,
                            name: name.clone(),
                            gate: lower_exec_expr(gate),
                        },
                        arcana_hir::HirBindLineKind::Assign { name, gate } => {
                            ExecBindLineKind::Assign {
                                name: name.clone(),
                                gate: lower_exec_expr(gate),
                            }
                        }
                        arcana_hir::HirBindLineKind::Require { expr } => {
                            ExecBindLineKind::Require {
                                expr: lower_exec_expr(expr),
                            }
                        }
                    },
                    modifier: line.modifier.as_ref().map(lower_headed_modifier_exec),
                })
                .collect(),
        }],
        HirStatementKind::Construct(region) => {
            vec![ExecStmt::Construct(lower_construct_region_exec(region))]
        }
        HirStatementKind::Record(region) => {
            vec![ExecStmt::Record(lower_record_region_exec(region))]
        }
        HirStatementKind::Array(region) => {
            vec![ExecStmt::Array(lower_array_region_exec(region))]
        }
        HirStatementKind::MemorySpec(spec) => {
            vec![ExecStmt::MemorySpec(lower_module_memory_spec_exec(spec))]
        }
    }
}

fn lower_exec_stmt_block_resolved_with_cleanup_candidates(
    statements: &[HirStatement],
    scope: &mut ResolvedRenderScope<'_>,
    inherited_cleanup_cleanup_footers: &[HirCleanupFooter],
) -> Result<LoweredExecBlock, String> {
    let mut lowered = LoweredExecBlock::default();
    for statement in statements {
        let exec = lower_exec_stmt_resolved(statement, scope, inherited_cleanup_cleanup_footers)?;
        lowered.cleanup_bindings.extend(exec.cleanup_bindings);
        lowered.statements.extend(exec.statements);
    }
    Ok(lowered)
}

fn lower_exec_stmt_resolved(
    statement: &HirStatement,
    scope: &mut ResolvedRenderScope<'_>,
    inherited_cleanup_cleanup_footers: &[HirCleanupFooter],
) -> Result<LoweredExecBlock, String> {
    let (lowered_statements, mut cleanup_bindings) = match &statement.kind {
        HirStatementKind::Let {
            mutable,
            name,
            value,
        } => {
            let destructuring_binding = binding_pattern_is_destructuring(name);
            if let Some(owner_activation) = resolve_owner_activation_expr(scope, value)? {
                if destructuring_binding {
                    return Err("owner activation bindings must use a simple name".to_string());
                }
                let lowered_context = owner_activation
                    .context
                    .map(|expr| lower_exec_expr_resolved(expr, scope));
                scope.value_scope.activate_owner(
                    &owner_activation.owner_local_name,
                    &owner_activation.owner_path,
                    &owner_activation.objects,
                    Some(name),
                );
                let object_binding_ids = owner_activation
                    .objects
                    .iter()
                    .filter_map(|(local_name, _)| {
                        scope
                            .value_scope
                            .binding_id_of(local_name)
                            .map(|binding_id| ExecNamedBindingId {
                                name: local_name.clone(),
                                binding_id,
                            })
                    })
                    .collect();
                (
                    vec![ExecStmt::ActivateOwner {
                        owner_path: owner_activation.owner_path,
                        owner_local_name: owner_activation.owner_local_name,
                        binding: Some(name.clone()),
                        object_binding_ids,
                        context: lowered_context,
                    }],
                    Vec::new(),
                )
            } else {
                let lowered_value = lower_exec_expr_resolved(value, scope);
                match parse_binding_pattern(name) {
                    Some(pattern) if matches!(pattern, BindingPattern::Pair(_, _)) => {
                        let value_ty = infer_expr_hir_type(scope, value).ok_or_else(|| {
                            "tuple destructuring requires a known pair type".to_string()
                        })?;
                        let temp_name = scope.value_scope.fresh_temp_name("tuple_let");
                        scope
                            .value_scope
                            .insert(temp_name.clone(), value_ty.clone());
                        let base_expr = ExecExpr::Path(vec![temp_name.clone()]);
                        let mut statements = vec![ExecStmt::Let {
                            binding_id: 0,
                            mutable: false,
                            name: temp_name,
                            value: lowered_value,
                        }];
                        let mut destructured = Vec::new();
                        collect_typed_binding_pattern_exec_lets(
                            &pattern,
                            &value_ty,
                            &base_expr,
                            &mut Vec::new(),
                            &mut destructured,
                            *mutable,
                            scope,
                        )?;
                        statements.extend(destructured);
                        (statements, Vec::new())
                    }
                    _ => {
                        let mut binding_id = 0;
                        if let Some(ty) = infer_expr_hir_type(scope, value) {
                            binding_id = scope.value_scope.insert(name.clone(), ty);
                        }
                        (
                            vec![ExecStmt::Let {
                                binding_id,
                                mutable: *mutable,
                                name: name.clone(),
                                value: lowered_value,
                            }],
                            Vec::new(),
                        )
                    }
                }
            }
        }
        HirStatementKind::Expr { expr } => {
            if let Some(owner_activation) = resolve_owner_activation_expr(scope, expr)? {
                let lowered_context = owner_activation
                    .context
                    .map(|value| lower_exec_expr_resolved(value, scope));
                scope.value_scope.activate_owner(
                    &owner_activation.owner_local_name,
                    &owner_activation.owner_path,
                    &owner_activation.objects,
                    None,
                );
                let object_binding_ids = owner_activation
                    .objects
                    .iter()
                    .filter_map(|(local_name, _)| {
                        scope
                            .value_scope
                            .binding_id_of(local_name)
                            .map(|binding_id| ExecNamedBindingId {
                                name: local_name.clone(),
                                binding_id,
                            })
                    })
                    .collect();
                (
                    vec![ExecStmt::ActivateOwner {
                        owner_path: owner_activation.owner_path,
                        owner_local_name: owner_activation.owner_local_name,
                        binding: None,
                        object_binding_ids,
                        context: lowered_context,
                    }],
                    Vec::new(),
                )
            } else {
                let statement_cleanup_cleanup_footers = effective_cleanup_cleanup_footers(
                    &statement.cleanup_footers,
                    inherited_cleanup_cleanup_footers,
                );
                (
                    vec![ExecStmt::Expr {
                        expr: lower_exec_expr_resolved(expr, scope),
                        cleanup_footers: lower_resolved_statement_cleanup_footers(
                            scope,
                            statement_cleanup_cleanup_footers,
                            Vec::new(),
                        )?,
                    }],
                    Vec::new(),
                )
            }
        }
        HirStatementKind::Return { value } => (
            vec![match value.as_ref() {
                Some(value) => ExecStmt::ReturnValue {
                    value: lower_exec_expr_resolved(value, scope),
                },
                None => ExecStmt::ReturnVoid,
            }],
            Vec::new(),
        ),
        HirStatementKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            let statement_cleanup_cleanup_footers = effective_cleanup_cleanup_footers(
                &statement.cleanup_footers,
                inherited_cleanup_cleanup_footers,
            );
            let mut then_scope = scope.clone();
            let then_block = lower_exec_stmt_block_resolved_with_cleanup_candidates(
                then_branch,
                &mut then_scope,
                statement_cleanup_cleanup_footers,
            )?;
            let else_branch = else_branch
                .as_ref()
                .map(|branch| {
                    let mut else_scope = scope.clone();
                    lower_exec_stmt_block_resolved_with_cleanup_candidates(
                        branch,
                        &mut else_scope,
                        statement_cleanup_cleanup_footers,
                    )
                })
                .transpose()?
                .unwrap_or_default();
            let mut rollup_bindings = then_block.cleanup_bindings;
            rollup_bindings.extend(else_branch.cleanup_bindings.iter().cloned());
            (
                vec![ExecStmt::If {
                    condition: lower_exec_expr_resolved(condition, scope),
                    then_branch: then_block.statements,
                    else_branch: else_branch.statements,
                    availability: statement
                        .availability
                        .iter()
                        .map(|attachment| {
                            lower_availability_attachment_exec_resolved(attachment, scope)
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                    cleanup_footers: lower_resolved_statement_cleanup_footers(
                        scope,
                        statement_cleanup_cleanup_footers,
                        rollup_bindings.clone(),
                    )?,
                }],
                rollup_bindings,
            )
        }
        HirStatementKind::While { condition, body } => {
            let statement_cleanup_cleanup_footers = effective_cleanup_cleanup_footers(
                &statement.cleanup_footers,
                inherited_cleanup_cleanup_footers,
            );
            let mut body_scope = scope.clone();
            let body_block = lower_exec_stmt_block_resolved_with_cleanup_candidates(
                body,
                &mut body_scope,
                statement_cleanup_cleanup_footers,
            )?;
            let cleanup_bindings = body_block.cleanup_bindings;
            (
                vec![ExecStmt::While {
                    condition: lower_exec_expr_resolved(condition, scope),
                    body: body_block.statements,
                    availability: statement
                        .availability
                        .iter()
                        .map(|attachment| {
                            lower_availability_attachment_exec_resolved(attachment, scope)
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                    cleanup_footers: lower_resolved_statement_cleanup_footers(
                        scope,
                        statement_cleanup_cleanup_footers,
                        cleanup_bindings.clone(),
                    )?,
                }],
                cleanup_bindings,
            )
        }
        HirStatementKind::For {
            binding,
            iterable,
            body,
        } => {
            let statement_cleanup_cleanup_footers = effective_cleanup_cleanup_footers(
                &statement.cleanup_footers,
                inherited_cleanup_cleanup_footers,
            );
            let mut body_scope = scope.clone();
            let mut rollup_bindings = Vec::new();
            let iterable_binding_ty = infer_iterable_binding_type(scope, iterable);
            let (binding_id, binding_name, prefix_statements) =
                if binding_pattern_is_destructuring(binding) {
                    let pattern = parse_binding_pattern(binding)
                        .ok_or_else(|| format!("invalid binding pattern `{binding}`"))?;
                    let value_ty = iterable_binding_ty.clone().ok_or_else(|| {
                        "tuple destructuring requires a known pair type".to_string()
                    })?;
                    let temp_name = body_scope.value_scope.fresh_temp_name("tuple_for");
                    body_scope
                        .value_scope
                        .insert(temp_name.clone(), value_ty.clone());
                    let base_expr = ExecExpr::Path(vec![temp_name.clone()]);
                    let mut lowered = Vec::new();
                    collect_typed_binding_pattern_exec_lets(
                        &pattern,
                        &value_ty,
                        &base_expr,
                        &mut Vec::new(),
                        &mut lowered,
                        false,
                        &mut body_scope,
                    )?;
                    let mut binding_names = Vec::new();
                    collect_binding_pattern_names(&pattern, &mut binding_names);
                    for binding_name in binding_names {
                        let Some(binding_id) = body_scope.value_scope.binding_id_of(&binding_name)
                        else {
                            continue;
                        };
                        let Some(binding_ty) =
                            body_scope.value_scope.type_of(&binding_name).cloned()
                        else {
                            continue;
                        };
                        push_cleanup_binding_candidate(
                            &mut rollup_bindings,
                            binding_name,
                            binding_id,
                            binding_ty,
                        );
                    }
                    (0, temp_name, lowered)
                } else {
                    let mut binding_id = 0;
                    if let Some(ty) = iterable_binding_ty {
                        binding_id = body_scope.value_scope.insert(binding.clone(), ty.clone());
                        push_cleanup_binding_candidate(
                            &mut rollup_bindings,
                            binding.clone(),
                            binding_id,
                            ty,
                        );
                    }
                    (binding_id, binding.clone(), Vec::new())
                };
            let body_block = lower_exec_stmt_block_resolved_with_cleanup_candidates(
                body,
                &mut body_scope,
                statement_cleanup_cleanup_footers,
            )?;
            rollup_bindings.extend(body_block.cleanup_bindings.iter().cloned());
            let mut lowered_body = prefix_statements;
            lowered_body.extend(body_block.statements);
            (
                vec![ExecStmt::For {
                    binding_id,
                    binding: binding_name,
                    iterable: lower_exec_expr_resolved(iterable, scope),
                    body: lowered_body,
                    availability: statement
                        .availability
                        .iter()
                        .map(|attachment| {
                            lower_availability_attachment_exec_resolved(attachment, scope)
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                    cleanup_footers: lower_resolved_statement_cleanup_footers(
                        scope,
                        statement_cleanup_cleanup_footers,
                        rollup_bindings.clone(),
                    )?,
                }],
                rollup_bindings,
            )
        }
        HirStatementKind::Defer { action } => (
            vec![ExecStmt::Defer(match action {
                arcana_hir::HirDeferAction::Expr { expr } => {
                    ExecDeferAction::Expr(lower_exec_expr_resolved(expr, scope))
                }
                arcana_hir::HirDeferAction::Reclaim { expr } => {
                    ExecDeferAction::Reclaim(lower_exec_expr_resolved(expr, scope))
                }
            })],
            Vec::new(),
        ),
        HirStatementKind::Reclaim { expr } => (
            vec![ExecStmt::Reclaim(lower_exec_expr_resolved(expr, scope))],
            Vec::new(),
        ),
        HirStatementKind::Break => (vec![ExecStmt::Break], Vec::new()),
        HirStatementKind::Continue => (vec![ExecStmt::Continue], Vec::new()),
        HirStatementKind::Assign { target, op, value } => {
            let lowered = ExecStmt::Assign {
                target: lower_assign_target_exec_resolved(target, scope),
                op: lower_assign_op(*op),
                value: lower_exec_expr_resolved(value, scope),
            };
            if matches!(op, HirAssignOp::Assign)
                && let HirAssignTarget::Name { text } = target
                && let Some(ty) = infer_expr_hir_type(scope, value)
            {
                scope.value_scope.insert(text.clone(), ty);
            }
            (vec![lowered], Vec::new())
        }
        HirStatementKind::Recycle {
            default_modifier,
            lines,
        } => {
            let mut region_scope = scope.clone();
            let default_modifier = default_modifier
                .as_ref()
                .map(|modifier| lower_headed_modifier_exec_resolved(modifier, &region_scope));
            let mut lowered_lines = Vec::new();
            for line in lines {
                let lowered_line = ExecRecycleLine {
                    kind: match &line.kind {
                        arcana_hir::HirRecycleLineKind::Expr { gate } => {
                            ExecRecycleLineKind::Expr {
                                gate: lower_exec_expr_resolved(gate, &region_scope),
                            }
                        }
                        arcana_hir::HirRecycleLineKind::Let {
                            mutable,
                            name,
                            gate,
                        } => ExecRecycleLineKind::Let {
                            mutable: *mutable,
                            name: name.clone(),
                            gate: lower_exec_expr_resolved(gate, &region_scope),
                        },
                        arcana_hir::HirRecycleLineKind::Assign { name, gate } => {
                            ExecRecycleLineKind::Assign {
                                name: name.clone(),
                                gate: lower_exec_expr_resolved(gate, &region_scope),
                            }
                        }
                    },
                    modifier: line.modifier.as_ref().map(|modifier| {
                        lower_headed_modifier_exec_resolved(modifier, &region_scope)
                    }),
                };
                match &line.kind {
                    arcana_hir::HirRecycleLineKind::Let { name, gate, .. } => {
                        if let Some(ty) = infer_headed_payload_binding_type(&region_scope, gate) {
                            region_scope.value_scope.insert(name.clone(), ty.clone());
                            scope.value_scope.insert(name.clone(), ty);
                        }
                    }
                    arcana_hir::HirRecycleLineKind::Assign { name, gate } => {
                        if region_scope.value_scope.contains(name)
                            && let Some(ty) = infer_headed_payload_binding_type(&region_scope, gate)
                        {
                            region_scope.value_scope.update_type(name, ty.clone());
                            scope.value_scope.update_type(name, ty);
                        }
                    }
                    arcana_hir::HirRecycleLineKind::Expr { .. } => {}
                }
                lowered_lines.push(lowered_line);
            }
            (
                vec![ExecStmt::Recycle {
                    default_modifier,
                    lines: lowered_lines,
                }],
                Vec::new(),
            )
        }
        HirStatementKind::Bind {
            default_modifier,
            lines,
        } => {
            let mut region_scope = scope.clone();
            let default_modifier = default_modifier
                .as_ref()
                .map(|modifier| lower_headed_modifier_exec_resolved(modifier, &region_scope));
            let mut lowered_lines = Vec::new();
            for line in lines {
                let lowered_line = ExecBindLine {
                    kind: match &line.kind {
                        arcana_hir::HirBindLineKind::Let {
                            mutable,
                            name,
                            gate,
                        } => ExecBindLineKind::Let {
                            mutable: *mutable,
                            name: name.clone(),
                            gate: lower_exec_expr_resolved(gate, &region_scope),
                        },
                        arcana_hir::HirBindLineKind::Assign { name, gate } => {
                            ExecBindLineKind::Assign {
                                name: name.clone(),
                                gate: lower_exec_expr_resolved(gate, &region_scope),
                            }
                        }
                        arcana_hir::HirBindLineKind::Require { expr } => {
                            ExecBindLineKind::Require {
                                expr: lower_exec_expr_resolved(expr, &region_scope),
                            }
                        }
                    },
                    modifier: line.modifier.as_ref().map(|modifier| {
                        lower_headed_modifier_exec_resolved(modifier, &region_scope)
                    }),
                };
                match &line.kind {
                    arcana_hir::HirBindLineKind::Let { name, gate, .. } => {
                        if let Some(ty) = infer_headed_payload_binding_type(&region_scope, gate) {
                            region_scope.value_scope.insert(name.clone(), ty.clone());
                            scope.value_scope.insert(name.clone(), ty);
                        }
                    }
                    arcana_hir::HirBindLineKind::Assign { name, gate } => {
                        if region_scope.value_scope.contains(name)
                            && let Some(ty) = infer_headed_payload_binding_type(&region_scope, gate)
                        {
                            region_scope.value_scope.update_type(name, ty.clone());
                            scope.value_scope.update_type(name, ty);
                        }
                    }
                    arcana_hir::HirBindLineKind::Require { .. } => {}
                }
                lowered_lines.push(lowered_line);
            }
            (
                vec![ExecStmt::Bind {
                    default_modifier,
                    lines: lowered_lines,
                }],
                Vec::new(),
            )
        }
        HirStatementKind::Construct(region) => {
            let lowered = ExecStmt::Construct(lower_construct_region_exec_resolved(region, scope));
            if let Some(arcana_hir::HirConstructDestination::Deliver { name }) = &region.destination
                && let Some(target_path) = flatten_callable_expr_path(&region.target)
                && let Some(ty) = resolve_construct_result_type_for_scope(scope, &target_path)
            {
                scope.value_scope.insert(name.clone(), ty);
            }
            (vec![lowered], Vec::new())
        }
        HirStatementKind::Record(region) => {
            let lowered = ExecStmt::Record(lower_record_region_exec_resolved(region, scope));
            if let Some(arcana_hir::HirConstructDestination::Deliver { name }) = &region.destination
                && let Some(target_path) = flatten_callable_expr_path(&region.target)
                && let Some(ty) = resolve_record_result_type_for_scope(scope, &target_path)
            {
                scope.value_scope.insert(name.clone(), ty);
            }
            (vec![lowered], Vec::new())
        }
        HirStatementKind::Array(region) => {
            let lowered = ExecStmt::Array(lower_array_region_exec_resolved(region, scope));
            if let Some(arcana_hir::HirConstructDestination::Deliver { name }) = &region.destination
                && let Some(target_path) = flatten_callable_expr_path(&region.target)
                && let Some(ty) = resolve_record_result_type_for_scope(scope, &target_path)
            {
                scope.value_scope.insert(name.clone(), ty);
            }
            (vec![lowered], Vec::new())
        }
        HirStatementKind::MemorySpec(spec) => (
            vec![ExecStmt::MemorySpec(ExecMemorySpecDecl {
                family: spec.family.as_str().to_string(),
                name: spec.name.clone(),
                default_modifier: spec
                    .default_modifier
                    .as_ref()
                    .map(|modifier| lower_headed_modifier_exec_resolved(modifier, scope)),
                details: spec
                    .details
                    .iter()
                    .map(|detail| ExecMemoryDetailLine {
                        key: detail.key.as_str().to_string(),
                        value: lower_exec_expr_resolved(&detail.value, scope),
                        modifier: detail
                            .modifier
                            .as_ref()
                            .map(|modifier| lower_headed_modifier_exec_resolved(modifier, scope)),
                    })
                    .collect(),
            })],
            Vec::new(),
        ),
    };
    for lowered_stmt in &lowered_statements {
        collect_lowered_cleanup_bindings(scope, lowered_stmt, &mut cleanup_bindings);
    }
    Ok(LoweredExecBlock {
        statements: lowered_statements,
        cleanup_bindings,
    })
}

fn is_routine_symbol(symbol: &HirSymbol) -> bool {
    matches!(
        symbol.kind,
        HirSymbolKind::Fn | HirSymbolKind::System | HirSymbolKind::Behavior | HirSymbolKind::Const
    )
}

fn lower_object_method_routines(module: &HirModuleSummary) -> Vec<IrRoutine> {
    module
        .symbols
        .iter()
        .enumerate()
        .flat_map(|(symbol_index, symbol)| {
            let HirSymbolBody::Object { methods, .. } = &symbol.body else {
                return Vec::new();
            };
            methods
                .iter()
                .enumerate()
                .filter(|(_, method)| is_routine_symbol(method))
                .map(|(method_index, method)| {
                    lower_routine(
                        &module.module_id,
                        routine_key_for_object_method(
                            &module.module_id,
                            symbol_index,
                            method_index,
                        ),
                        method,
                        None,
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn lower_object_method_routines_resolved(
    workspace: &HirWorkspaceSummary,
    package: &HirWorkspacePackage,
    resolved_module: &HirResolvedModule,
    module: &HirModuleSummary,
) -> Result<Vec<IrRoutine>, String> {
    module
        .symbols
        .iter()
        .enumerate()
        .flat_map(|(symbol_index, symbol)| {
            let HirSymbolBody::Object { methods, .. } = &symbol.body else {
                return Vec::new();
            };
            methods
                .iter()
                .enumerate()
                .filter(|(_, method)| is_routine_symbol(method))
                .map(move |(method_index, method)| {
                    lower_routine_resolved(
                        workspace,
                        package,
                        resolved_module,
                        module,
                        &module.module_id,
                        routine_key_for_object_method(
                            &module.module_id,
                            symbol_index,
                            method_index,
                        ),
                        method,
                        None,
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn lower_owner_decl(module: &HirModuleSummary, symbol: &HirSymbol) -> Option<IrOwnerDecl> {
    let HirSymbolBody::Owner {
        objects,
        context_type,
        exits,
    } = &symbol.body
    else {
        return None;
    };
    Some(IrOwnerDecl {
        package_id: module
            .module_id
            .split('.')
            .next()
            .unwrap_or(&module.module_id)
            .to_string(),
        module_id: module.module_id.clone(),
        owner_path: canonical_symbol_path(&module.module_id, &symbol.name),
        owner_name: symbol.name.clone(),
        context_type: context_type.as_ref().map(lower_symbol_routine_type),
        objects: objects
            .iter()
            .map(|object| IrOwnerObject {
                type_path: object.type_path.clone(),
                local_name: object.local_name.clone(),
                init_routine_key: None,
                init_with_context_routine_key: None,
                resume_routine_key: None,
                resume_with_context_routine_key: None,
            })
            .collect(),
        exits: exits
            .iter()
            .map(|owner_exit| IrOwnerExit {
                name: owner_exit.name.clone(),
                condition: lower_exec_expr(&owner_exit.condition),
                retains: owner_exit.retains.clone(),
            })
            .collect(),
    })
}

fn resolve_object_lifecycle_routine_keys(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    type_path: &[String],
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    let Some(resolved) = lookup_symbol_path(workspace, resolved_module, type_path) else {
        return (None, None, None, None);
    };
    let HirSymbolBody::Object { methods, .. } = &resolved.symbol.body else {
        return (None, None, None, None);
    };

    let mut init_routine_key = None;
    let mut init_with_context_routine_key = None;
    let mut resume_routine_key = None;
    let mut resume_with_context_routine_key = None;

    for (method_index, method) in methods.iter().enumerate() {
        let slot = match method.name.as_str() {
            "init" => {
                if method.params.len() == 1 {
                    &mut init_routine_key
                } else if method.params.len() == 2 {
                    &mut init_with_context_routine_key
                } else {
                    continue;
                }
            }
            "resume" => {
                if method.params.len() == 1 {
                    &mut resume_routine_key
                } else if method.params.len() == 2 {
                    &mut resume_with_context_routine_key
                } else {
                    continue;
                }
            }
            _ => continue,
        };
        if method.is_async || !method.type_params.is_empty() {
            continue;
        }
        let Some(receiver) = method.params.first() else {
            continue;
        };
        if receiver.mode != Some(arcana_hir::HirParamMode::Edit) {
            continue;
        }
        if let Some(context) = method.params.get(1)
            && context.mode != Some(arcana_hir::HirParamMode::Read)
        {
            continue;
        }
        if slot.is_none() {
            *slot = Some(routine_key_for_object_method(
                resolved.module_id,
                resolved.symbol_index,
                method_index,
            ));
        }
    }

    (
        init_routine_key,
        init_with_context_routine_key,
        resume_routine_key,
        resume_with_context_routine_key,
    )
}

fn lower_owner_decl_resolved(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module: &HirModuleSummary,
    symbol: &HirSymbol,
) -> Result<Option<IrOwnerDecl>, String> {
    let HirSymbolBody::Owner {
        objects,
        context_type,
        exits,
    } = &symbol.body
    else {
        return Ok(None);
    };
    let scope = ResolvedRenderScope::new(
        workspace,
        resolved_module,
        symbol.where_clause.clone(),
        &symbol.type_params,
    );
    Ok(Some(IrOwnerDecl {
        package_id: resolved_module.package_id.clone(),
        module_id: module.module_id.clone(),
        owner_path: canonical_symbol_path(&module.module_id, &symbol.name),
        owner_name: symbol.name.clone(),
        context_type: context_type
            .as_ref()
            .map(|ty| lower_resolved_routine_type(workspace, resolved_module, ty)),
        objects: objects
            .iter()
            .map(|object| {
                let canonical = lookup_symbol_path(workspace, resolved_module, &object.type_path)
                    .map(|resolved| {
                        canonical_symbol_path(resolved.module_id, &resolved.symbol.name)
                    })
                    .unwrap_or_else(|| object.type_path.clone());
                let (
                    init_routine_key,
                    init_with_context_routine_key,
                    resume_routine_key,
                    resume_with_context_routine_key,
                ) = resolve_object_lifecycle_routine_keys(
                    workspace,
                    resolved_module,
                    &object.type_path,
                );
                IrOwnerObject {
                    type_path: canonical,
                    local_name: object.local_name.clone(),
                    init_routine_key,
                    init_with_context_routine_key,
                    resume_routine_key,
                    resume_with_context_routine_key,
                }
            })
            .collect(),
        exits: exits
            .iter()
            .map(|owner_exit| IrOwnerExit {
                name: owner_exit.name.clone(),
                condition: lower_exec_expr_resolved(&owner_exit.condition, &scope),
                retains: owner_exit.retains.clone(),
            })
            .collect(),
    }))
}

fn lower_routine(
    module_id: &str,
    routine_key: String,
    symbol: &HirSymbol,
    impl_decl: Option<&arcana_hir::HirImplDecl>,
) -> IrRoutine {
    let type_params = impl_decl
        .into_iter()
        .flat_map(|decl| decl.type_params.iter().cloned())
        .chain(symbol.type_params.iter().cloned())
        .collect::<Vec<_>>();
    IrRoutine {
        package_id: module_id.split('.').next().unwrap_or(module_id).to_string(),
        module_id: module_id.to_string(),
        routine_key,
        symbol_name: symbol.name.clone(),
        symbol_kind: symbol.kind.as_str().to_string(),
        exported: symbol.exported,
        is_async: symbol.is_async,
        type_params,
        behavior_attrs: lower_behavior_attrs(symbol),
        params: lower_routine_params(symbol),
        return_type: symbol.return_type.as_ref().map(lower_symbol_routine_type),
        intrinsic_impl: symbol.intrinsic_impl.clone(),
        native_impl: symbol.native_impl.clone(),
        impl_target_type: impl_decl.map(|decl| lower_symbol_routine_type(&decl.target_type)),
        impl_trait_path: impl_decl
            .and_then(|decl| decl.trait_path.as_ref().map(canonical_impl_trait_path)),
        availability: symbol
            .availability
            .iter()
            .map(lower_availability_attachment_exec)
            .collect(),
        inline_hint: symbol_has_builtin_foreword(symbol, "inline"),
        cold_hint: symbol_has_builtin_foreword(symbol, "cold"),
        cleanup_footers: symbol.cleanup_footers.iter().map(lower_rollup).collect(),
        statements: lower_exec_stmt_block(&symbol.statements),
    }
}

fn lower_routine_resolved(
    workspace: &HirWorkspaceSummary,
    package: &HirWorkspacePackage,
    resolved_module: &HirResolvedModule,
    module: &HirModuleSummary,
    module_id: &str,
    routine_key: String,
    symbol: &HirSymbol,
    impl_decl: Option<&arcana_hir::HirImplDecl>,
) -> Result<IrRoutine, String> {
    let type_params = impl_decl
        .into_iter()
        .flat_map(|decl| decl.type_params.iter().cloned())
        .chain(symbol.type_params.iter().cloned())
        .collect::<Vec<_>>();
    let mut scope = ResolvedRenderScope::new(
        workspace,
        resolved_module,
        symbol.where_clause.clone(),
        &type_params,
    );
    for param in &symbol.params {
        scope
            .value_scope
            .insert(param.name.clone(), param.ty.clone());
    }
    let lowered_block = lower_exec_stmt_block_resolved_with_cleanup_candidates(
        &symbol.statements,
        &mut scope,
        &symbol.cleanup_footers,
    )?;
    let routine = IrRoutine {
        package_id: package.package_id.clone(),
        module_id: module_id.to_string(),
        routine_key,
        symbol_name: symbol.name.clone(),
        symbol_kind: symbol.kind.as_str().to_string(),
        exported: symbol.exported
            || impl_decl.is_some_and(|decl| {
                impl_target_is_public_from_package(workspace, package, module, &decl.target_type)
            }),
        is_async: symbol.is_async,
        type_params,
        behavior_attrs: lower_behavior_attrs(symbol),
        params: lower_routine_params_resolved(workspace, resolved_module, symbol, &scope),
        return_type: symbol
            .return_type
            .as_ref()
            .map(|ty| lower_resolved_routine_type(workspace, resolved_module, ty)),
        intrinsic_impl: symbol.intrinsic_impl.clone(),
        native_impl: symbol.native_impl.clone(),
        impl_target_type: impl_decl
            .map(|decl| lower_resolved_routine_type(workspace, resolved_module, &decl.target_type)),
        impl_trait_path: impl_decl
            .and_then(|decl| decl.trait_path.as_ref().map(canonical_impl_trait_path)),
        availability: symbol
            .availability
            .iter()
            .map(|attachment| lower_availability_attachment_exec_resolved(attachment, &scope))
            .collect::<Result<Vec<_>, _>>()?,
        inline_hint: symbol_has_builtin_foreword(symbol, "inline"),
        cold_hint: symbol_has_builtin_foreword(symbol, "cold"),
        cleanup_footers: lower_resolved_symbol_cleanup_footers(
            &scope,
            symbol,
            lowered_block.cleanup_bindings,
        )?,
        statements: lowered_block.statements,
    };
    scope.finish()?;
    Ok(routine)
}

fn lower_package(package: &HirPackageSummary) -> IrPackage {
    let modules = package
        .modules
        .iter()
        .map(|module| {
            let lowered = lower_module_summary(module);
            IrPackageModule {
                package_id: package.package_name.clone(),
                module_id: module.module_id.clone(),
                symbol_count: lowered.symbol_count,
                item_count: lowered.item_count,
                line_count: module.line_count,
                non_empty_line_count: module.non_empty_line_count,
                directive_rows: module
                    .directives
                    .iter()
                    .map(|directive| {
                        render_directive_row(
                            &module.module_id,
                            directive.kind,
                            &directive.path,
                            &directive.alias,
                        )
                    })
                    .collect(),
                lang_item_rows: module
                    .lang_items
                    .iter()
                    .map(|item| render_lang_item_row(&module.module_id, &item.name, &item.target))
                    .chain(module_struct_bitfield_layout_rows(module))
                    .collect(),
                exported_surface_rows: {
                    let mut rows = module.summary_surface_rows();
                    rows.extend(module.memory_specs.iter().filter_map(|spec| {
                        render_memory_spec_surface_row(&lower_module_memory_spec_exec(spec)).ok()
                    }));
                    rows
                },
            }
        })
        .collect::<Vec<_>>();
    let dependency_rows = package
        .dependency_edges
        .iter()
        .map(render_dependency_row)
        .collect::<Vec<_>>();
    let entrypoints = package
        .modules
        .iter()
        .flat_map(|module| {
            module.symbols.iter().filter_map(|symbol| {
                let is_entry = symbol.kind == HirSymbolKind::System
                    || symbol.kind == HirSymbolKind::Behavior
                    || is_runtime_main_entry_symbol(
                        &package.package_name,
                        &module.module_id,
                        symbol,
                    );
                if !is_entry {
                    return None;
                }
                Some(IrEntrypoint {
                    package_id: package.package_name.clone(),
                    module_id: module.module_id.clone(),
                    symbol_name: symbol.name.clone(),
                    symbol_kind: symbol.kind.as_str().to_string(),
                    is_async: symbol.is_async,
                    exported: symbol.exported,
                })
            })
        })
        .collect::<Vec<_>>();
    let routines =
        package
            .modules
            .iter()
            .flat_map(|module| {
                let mut routines = module
                    .symbols
                    .iter()
                    .enumerate()
                    .filter(|(_, symbol)| is_routine_symbol(symbol))
                    .map(|(symbol_index, symbol)| {
                        lower_routine(
                            &module.module_id,
                            routine_key_for_symbol(&module.module_id, symbol_index),
                            symbol,
                            None,
                        )
                    })
                    .collect::<Vec<_>>();
                routines.extend(module.impls.iter().enumerate().flat_map(
                    |(impl_index, impl_decl)| {
                        impl_decl
                            .methods
                            .iter()
                            .enumerate()
                            .filter(|(_, symbol)| is_routine_symbol(symbol))
                            .map(move |(method_index, symbol)| {
                                lower_routine(
                                    &module.module_id,
                                    routine_key_for_impl_method(
                                        &module.module_id,
                                        impl_index,
                                        method_index,
                                    ),
                                    symbol,
                                    Some(impl_decl),
                                )
                            })
                    },
                ));
                routines.extend(lower_object_method_routines(module));
                routines
            })
            .collect::<Vec<_>>();
    let owners = package
        .modules
        .iter()
        .flat_map(|module| {
            module
                .symbols
                .iter()
                .filter_map(|symbol| lower_owner_decl(module, symbol))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let native_callbacks = package
        .modules
        .iter()
        .flat_map(|module| {
            module
                .native_callbacks
                .iter()
                .map(|callback| IrNativeCallbackDecl {
                    package_id: package.package_name.clone(),
                    module_id: module.module_id.clone(),
                    name: callback.name.clone(),
                    params: callback
                        .params
                        .iter()
                        .map(|param| IrRoutineParam {
                            binding_id: 0,
                            mode: param.mode.as_ref().map(|mode| mode.as_str().to_string()),
                            name: param.name.clone(),
                            ty: lower_symbol_routine_type(&param.ty),
                        })
                        .collect(),
                    return_type: callback.return_type.as_ref().map(lower_symbol_routine_type),
                    callback_type: callback
                        .callback_type
                        .as_ref()
                        .map(lower_symbol_routine_type),
                    target: callback.target.clone(),
                    target_routine_key: None,
                })
        })
        .collect::<Vec<_>>();
    let shackle_decls = package
        .modules
        .iter()
        .flat_map(|module| {
            module.shackle_decls.iter().map(|decl| IrShackleDecl {
                package_id: package.package_name.clone(),
                module_id: module.module_id.clone(),
                exported: decl.exported,
                kind: decl.kind.as_str().to_string(),
                name: decl.name.clone(),
                params: decl
                    .params
                    .iter()
                    .map(|param| IrRoutineParam {
                        binding_id: 0,
                        mode: param.mode.as_ref().map(|mode| mode.as_str().to_string()),
                        name: param.name.clone(),
                        ty: lower_symbol_routine_type(&param.ty),
                    })
                    .collect(),
                return_type: decl.return_type.as_ref().map(lower_symbol_routine_type),
                callback_type: decl.callback_type.as_ref().map(lower_symbol_routine_type),
                binding: decl.binding.clone(),
                body_entries: decl.body_entries.clone(),
                raw_layout: decl.raw_layout.clone(),
                import_target: decl
                    .import_target
                    .as_ref()
                    .map(|target| IrShackleImportTarget {
                        library: target.library.clone(),
                        symbol: target.symbol.clone(),
                        abi: target.abi.clone(),
                    }),
                thunk_target: decl
                    .thunk_target
                    .as_ref()
                    .map(|target| IrShackleThunkTarget {
                        target: target.target.clone(),
                        abi: target.abi.clone(),
                    }),
                surface_text: decl.surface_text.clone(),
            })
        })
        .collect::<Vec<_>>();

    let mut lowered = IrPackage {
        package_id: package.package_name.clone(),
        package_name: package.package_name.clone(),
        root_module_id: package.package_name.clone(),
        direct_deps: Vec::new(),
        direct_dep_ids: Vec::new(),
        package_display_names: BTreeMap::from([(
            package.package_name.clone(),
            package.package_name.clone(),
        )]),
        package_direct_dep_ids: BTreeMap::from([(package.package_name.clone(), BTreeMap::new())]),
        modules,
        dependency_edge_count: package.dependency_edges.len(),
        dependency_rows,
        exported_surface_rows: package.summary_surface_rows(),
        runtime_requirements: Vec::new(),
        foreword_index: Vec::new(),
        foreword_registrations: Vec::new(),
        entrypoints,
        routines,
        native_callbacks,
        shackle_decls,
        owners,
    };
    lowered.runtime_requirements = derive_runtime_requirements(&lowered);
    lowered
}

fn retarget_package_identity(package: &mut IrPackage, package_id: &str) {
    let old_package_id = package.package_id.clone();
    package.package_id = package_id.to_string();
    if let Some(display_name) = package.package_display_names.remove(&old_package_id) {
        package
            .package_display_names
            .insert(package_id.to_string(), display_name);
    }
    if let Some(dep_ids) = package.package_direct_dep_ids.remove(&old_package_id) {
        package
            .package_direct_dep_ids
            .insert(package_id.to_string(), dep_ids);
    }
    for module in &mut package.modules {
        module.package_id = package_id.to_string();
    }
    for entrypoint in &mut package.entrypoints {
        entrypoint.package_id = package_id.to_string();
    }
    for routine in &mut package.routines {
        routine.package_id = package_id.to_string();
    }
    for callback in &mut package.native_callbacks {
        callback.package_id = package_id.to_string();
    }
    for decl in &mut package.shackle_decls {
        decl.package_id = package_id.to_string();
    }
    for owner in &mut package.owners {
        owner.package_id = package_id.to_string();
    }
}

#[cfg(test)]
fn lower_workspace_package(package: &HirWorkspacePackage) -> IrPackage {
    let mut lowered = lower_package(&package.summary);
    retarget_package_identity(&mut lowered, &package.package_id);
    lowered.direct_deps = package.direct_deps.iter().cloned().collect();
    lowered.direct_dep_ids = resolved_direct_dep_ids(package);
    lowered
        .package_display_names
        .extend(resolved_direct_dep_display_names(package));
    lowered.package_direct_dep_ids.insert(
        package.package_id.clone(),
        resolved_direct_dep_package_ids(package),
    );
    let mut isolated_workspace = HirWorkspaceSummary::default();
    isolated_workspace
        .packages
        .insert(package.package_id.clone(), package.clone());
    lowered.foreword_index = lower_package_foreword_index(&isolated_workspace, package);
    lowered.foreword_registrations = lower_package_foreword_registrations(package);
    lowered
}

pub fn lower_workspace_package_with_resolution(
    workspace: &HirWorkspaceSummary,
    resolved_workspace: &HirResolvedWorkspace,
    package: &HirWorkspacePackage,
) -> Result<IrPackage, String> {
    let mut lowered = lower_package(&package.summary);
    retarget_package_identity(&mut lowered, &package.package_id);
    lowered.direct_deps = resolved_direct_deps(package);
    lowered.direct_dep_ids = resolved_direct_dep_ids(package);
    lowered
        .package_display_names
        .extend(resolved_direct_dep_display_names(package));
    lowered.package_direct_dep_ids.insert(
        package.package_id.clone(),
        resolved_direct_dep_package_ids(package),
    );
    lowered.dependency_rows = package
        .summary
        .dependency_edges
        .iter()
        .map(|edge| render_resolved_dependency_row(package, edge))
        .collect();
    lowered.dependency_edge_count = lowered.dependency_rows.len();
    lowered.foreword_index = lower_package_foreword_index(workspace, package);
    lowered.foreword_registrations = lower_package_foreword_registrations(package);
    let Some(resolved_package) = resolved_workspace.package_by_id(&package.package_id) else {
        return Ok(lowered);
    };
    lowered.routines = package
        .summary
        .modules
        .iter()
        .map(|module| {
            let Some(resolved_module) = resolved_package.module(&module.module_id) else {
                return Ok(Vec::new());
            };
            let mut routines = module
                .symbols
                .iter()
                .enumerate()
                .filter(|(_, symbol)| is_routine_symbol(symbol))
                .map(|(symbol_index, symbol)| {
                    lower_routine_resolved(
                        workspace,
                        package,
                        resolved_module,
                        module,
                        &module.module_id,
                        routine_key_for_symbol(&module.module_id, symbol_index),
                        symbol,
                        None,
                    )
                })
                .collect::<Result<Vec<_>, String>>()?;
            routines.extend(
                module
                    .impls
                    .iter()
                    .enumerate()
                    .flat_map(|(impl_index, impl_decl)| {
                        impl_decl
                            .methods
                            .iter()
                            .enumerate()
                            .filter(|(_, symbol)| is_routine_symbol(symbol))
                            .map(move |(method_index, symbol)| {
                                lower_routine_resolved(
                                    workspace,
                                    package,
                                    resolved_module,
                                    module,
                                    &module.module_id,
                                    routine_key_for_impl_method(
                                        &module.module_id,
                                        impl_index,
                                        method_index,
                                    ),
                                    symbol,
                                    Some(impl_decl),
                                )
                            })
                    })
                    .collect::<Result<Vec<_>, String>>()?,
            );
            routines.extend(lower_object_method_routines_resolved(
                workspace,
                package,
                resolved_module,
                module,
            )?);
            Ok(routines)
        })
        .collect::<Result<Vec<_>, String>>()?
        .into_iter()
        .flatten()
        .collect();
    lowered.native_callbacks = package
        .summary
        .modules
        .iter()
        .map(|module| {
            let Some(resolved_module) = resolved_package.module(&module.module_id) else {
                return Ok(Vec::new());
            };
            module
                .native_callbacks
                .iter()
                .map(|callback| {
                    let Some(symbol_ref) =
                        lookup_symbol_path(workspace, resolved_module, &callback.target)
                    else {
                        return Err(format!(
                            "native callback `{}` in module `{}` targets unresolved path `{}`",
                            callback.name,
                            module.module_id,
                            callback.target.join(".")
                        ));
                    };
                    if !is_routine_symbol(symbol_ref.symbol) {
                        return Err(format!(
                            "native callback `{}` in module `{}` must target a routine path, found `{}`",
                            callback.name, module.module_id, callback.target.join(".")
                        ));
                    }
                    let (params, return_type) =
                        lower_native_callback_signature(workspace, resolved_module, callback)?;
                    Ok(IrNativeCallbackDecl {
                        package_id: package.package_id.clone(),
                        module_id: module.module_id.clone(),
                        name: callback.name.clone(),
                        params,
                        return_type,
                        callback_type: callback.callback_type.as_ref().map(|ty| {
                            lower_resolved_routine_type(workspace, resolved_module, ty)
                        }),
                        target: callback.target.clone(),
                        target_routine_key: Some(routine_key_for_symbol(
                            symbol_ref.module_id,
                            symbol_ref.symbol_index,
                        )),
                    })
                })
                .collect::<Result<Vec<_>, String>>()
        })
        .collect::<Result<Vec<_>, String>>()?
        .into_iter()
        .flatten()
        .collect();
    lowered.shackle_decls = package
        .summary
        .modules
        .iter()
        .map(|module| {
            let Some(resolved_module) = resolved_package.module(&module.module_id) else {
                return Ok(Vec::new());
            };
            Ok(module
                .shackle_decls
                .iter()
                .map(|decl| IrShackleDecl {
                    package_id: package.package_id.clone(),
                    module_id: module.module_id.clone(),
                    exported: decl.exported,
                    kind: decl.kind.as_str().to_string(),
                    name: decl.name.clone(),
                    params: decl
                        .params
                        .iter()
                        .map(|param| IrRoutineParam {
                            binding_id: 0,
                            mode: param.mode.map(|mode| mode.as_str().to_string()),
                            name: param.name.clone(),
                            ty: lower_resolved_routine_type(workspace, resolved_module, &param.ty),
                        })
                        .collect(),
                    return_type: decl
                        .return_type
                        .as_ref()
                        .map(|ty| lower_resolved_routine_type(workspace, resolved_module, ty)),
                    callback_type: decl
                        .callback_type
                        .as_ref()
                        .map(|ty| lower_resolved_routine_type(workspace, resolved_module, ty)),
                    binding: decl.binding.clone(),
                    body_entries: decl.body_entries.clone(),
                    raw_layout: decl.raw_layout.clone(),
                    import_target: decl.import_target.as_ref().map(|target| {
                        IrShackleImportTarget {
                            library: target.library.clone(),
                            symbol: target.symbol.clone(),
                            abi: target.abi.clone(),
                        }
                    }),
                    thunk_target: decl
                        .thunk_target
                        .as_ref()
                        .map(|target| IrShackleThunkTarget {
                            target: target.target.clone(),
                            abi: target.abi.clone(),
                        }),
                    surface_text: decl.surface_text.clone(),
                })
                .collect::<Vec<_>>())
        })
        .collect::<Result<Vec<_>, String>>()?
        .into_iter()
        .flatten()
        .collect();
    if lowered.shackle_decls.iter().any(|decl| {
        matches!(
            decl.kind.as_str(),
            "type" | "struct" | "union" | "callback" | "flags"
        ) && decl.raw_layout.is_none()
            || matches!(decl.kind.as_str(), "import fn" | "import_fn")
                && decl.import_target.is_none()
            || decl.kind == "thunk" && decl.thunk_target.is_none()
    }) {
        populate_typed_shackle_metadata(&mut lowered)?;
    }
    lowered.owners = package
        .summary
        .modules
        .iter()
        .map(|module| {
            let Some(resolved_module) = resolved_package.module(&module.module_id) else {
                return Ok(Vec::new());
            };
            module
                .symbols
                .iter()
                .map(|symbol| lower_owner_decl_resolved(workspace, resolved_module, module, symbol))
                .collect::<Result<Vec<_>, String>>()
                .map(|owners| owners.into_iter().flatten().collect::<Vec<_>>())
        })
        .collect::<Result<Vec<_>, String>>()?
        .into_iter()
        .flatten()
        .collect();
    let module_surface_rows = package
        .summary
        .modules
        .iter()
        .map(|module| {
            let rows = resolved_package
                .module(&module.module_id)
                .map(|_| resolved_module_exported_surface_rows(workspace, package, module))
                .unwrap_or_else(|| module.summary_surface_rows());
            (module.module_id.clone(), rows)
        })
        .collect::<BTreeMap<_, _>>();
    let module_lang_item_rows = package
        .summary
        .modules
        .iter()
        .map(|module| {
            let rows = resolved_package
                .module(&module.module_id)
                .map(|resolved_module| {
                    resolved_module_lang_item_rows(workspace, resolved_module, module)
                })
                .unwrap_or_else(|| {
                    let mut rows = module
                        .lang_items
                        .iter()
                        .map(|item| {
                            render_lang_item_row(&module.module_id, &item.name, &item.target)
                        })
                        .collect::<Vec<_>>();
                    rows.extend(module_struct_bitfield_layout_rows(module));
                    rows
                });
            (module.module_id.clone(), rows)
        })
        .collect::<BTreeMap<_, _>>();
    for module in &mut lowered.modules {
        if let Some(rows) = module_surface_rows.get(&module.module_id) {
            module.exported_surface_rows = rows.clone();
        }
        if let Some(rows) = module_lang_item_rows.get(&module.module_id) {
            module.lang_item_rows = rows.clone();
        }
    }
    lowered.exported_surface_rows = package
        .summary
        .modules
        .iter()
        .flat_map(|module| {
            module_surface_rows
                .get(&module.module_id)
                .into_iter()
                .flatten()
                .map(|row| format!("module={}:{}", module.module_id, row))
                .collect::<Vec<_>>()
        })
        .collect();
    lowered.runtime_requirements = derive_runtime_requirements(&lowered);
    Ok(lowered)
}

fn populate_typed_shackle_metadata(package: &mut IrPackage) -> Result<(), String> {
    let snapshot = package.shackle_decls.clone();
    let decls_by_id = snapshot
        .iter()
        .filter_map(|decl| binding_layout_id_for_decl(decl).map(|layout_id| (layout_id, decl)))
        .collect::<BTreeMap<_, _>>();
    let mut builder = IrBindingLayoutBuilder {
        decls_by_id,
        built: BTreeMap::new(),
        building: BTreeSet::new(),
    };
    let mut raw_layouts = BTreeMap::<usize, ArcanaCabiBindingLayout>::new();
    let mut import_targets = BTreeMap::<usize, IrShackleImportTarget>::new();
    let mut thunk_targets = BTreeMap::<usize, IrShackleThunkTarget>::new();

    for (index, decl) in snapshot.iter().enumerate() {
        if let Some(layout_id) = binding_layout_id_for_decl(decl) {
            builder.build(&layout_id)?;
            let layout = builder.built.get(&layout_id).cloned().ok_or_else(|| {
                format!("missing typed raw layout `{layout_id}` after IR lowering")
            })?;
            raw_layouts.insert(index, layout);
        }
        match decl.kind.as_str() {
            "import fn" | "import_fn" => {
                if let Some(binding) = decl.binding.as_deref() {
                    import_targets.insert(index, parse_shackle_import_target(binding)?);
                }
            }
            "thunk" => {
                if let Some(binding) = decl.binding.as_ref() {
                    thunk_targets.insert(
                        index,
                        IrShackleThunkTarget {
                            target: binding.clone(),
                            abi: "system".to_string(),
                        },
                    );
                }
            }
            _ => {}
        }
    }

    for (index, decl) in package.shackle_decls.iter_mut().enumerate() {
        decl.raw_layout = raw_layouts.get(&index).cloned();
        decl.import_target = import_targets.get(&index).cloned();
        decl.thunk_target = thunk_targets.get(&index).cloned();
    }
    Ok(())
}

struct ParsedShackleField {
    name: String,
    ty: ArcanaCabiBindingRawType,
    scalar: Option<ArcanaCabiBindingScalarType>,
    bit_width: Option<u16>,
}

struct IrBindingLayoutBuilder<'a> {
    decls_by_id: BTreeMap<String, &'a IrShackleDecl>,
    built: BTreeMap<String, ArcanaCabiBindingLayout>,
    building: BTreeSet<String>,
}

impl IrBindingLayoutBuilder<'_> {
    fn build(&mut self, layout_id: &str) -> Result<(), String> {
        if self.built.contains_key(layout_id) {
            return Ok(());
        }
        let Some(decl) = self.decls_by_id.get(layout_id).copied() else {
            return Ok(());
        };
        if !self.building.insert(layout_id.to_string()) {
            return Err(format!(
                "recursive shackle raw layout cycle at `{layout_id}`"
            ));
        }
        let layout = match decl.kind.as_str() {
            "type" => self.build_type_layout(decl)?,
            "struct" => self.build_struct_layout(decl)?,
            "union" => self.build_union_layout(decl)?,
            "callback" => self.build_callback_layout(decl)?,
            "flags" => self.build_flags_layout(decl)?,
            other => {
                return Err(format!(
                    "unsupported shackle raw layout declaration kind `{other}` for `{layout_id}`"
                ));
            }
        };
        self.building.remove(layout_id);
        self.built.insert(layout_id.to_string(), layout);
        Ok(())
    }

    fn build_type_layout(
        &mut self,
        decl: &IrShackleDecl,
    ) -> Result<ArcanaCabiBindingLayout, String> {
        let layout_id = binding_layout_id(&decl.module_id, &decl.name);
        let binding = decl
            .binding
            .as_deref()
            .ok_or_else(|| format!("shackle type `{layout_id}` is missing a raw binding target"))?;
        if let Some((element_type, len)) =
            parse_fixed_array_type_expr(binding, &decl.module_id, self)?
        {
            let (element_size, element_align) = self.raw_type_size_align(&element_type)?;
            return Ok(ArcanaCabiBindingLayout {
                layout_id,
                size: element_size.saturating_mul(len),
                align: element_align,
                kind: ArcanaCabiBindingLayoutKind::Array { element_type, len },
            });
        }
        let target = parse_shackle_raw_type(binding, &decl.module_id, self)?;
        let vtable_layout_id = companion_vtable_layout_id(&decl.module_id, &decl.name);
        if self.decls_by_id.contains_key(&vtable_layout_id)
            && matches!(target, ArcanaCabiBindingRawType::Pointer { .. })
        {
            self.build(&vtable_layout_id)?;
            let size = std::mem::size_of::<usize>();
            return Ok(ArcanaCabiBindingLayout {
                layout_id,
                size,
                align: size,
                kind: ArcanaCabiBindingLayoutKind::Interface {
                    iid: None,
                    vtable_layout_id: Some(vtable_layout_id),
                },
            });
        }
        let (size, align) = self.raw_type_size_align(&target)?;
        Ok(ArcanaCabiBindingLayout {
            layout_id,
            size,
            align,
            kind: ArcanaCabiBindingLayoutKind::Alias { target },
        })
    }

    fn build_struct_layout(
        &mut self,
        decl: &IrShackleDecl,
    ) -> Result<ArcanaCabiBindingLayout, String> {
        let mut fields = Vec::new();
        let mut offset = 0usize;
        let mut max_align = 1usize;
        let mut active_bitfield: Option<(ArcanaCabiBindingScalarType, usize, usize, usize)> = None;
        for line in &decl.body_entries {
            let parsed = parse_shackle_struct_field(line, &decl.module_id, self)?;
            if let Some(bit_width) = parsed.bit_width {
                let scalar = parsed.scalar.ok_or_else(|| {
                    format!(
                        "bitfield `{}` on `{}` must use a fixed-width integer base type",
                        parsed.name, decl.name
                    )
                })?;
                let storage_size = scalar.size_bytes();
                let storage_align = scalar.align_bytes();
                let storage_bits = storage_size * 8;
                let (storage_offset, next_bit_offset, total_used_bits) =
                    if let Some((active_scalar, current_offset, current_bit_offset, used_bits)) =
                        active_bitfield
                    {
                        if active_scalar == scalar
                            && current_bit_offset + usize::from(bit_width) <= storage_bits
                        {
                            (
                                current_offset,
                                current_bit_offset,
                                used_bits + usize::from(bit_width),
                            )
                        } else {
                            offset = align_up(offset, storage_align);
                            let start = offset;
                            offset += storage_size;
                            (start, 0, usize::from(bit_width))
                        }
                    } else {
                        offset = align_up(offset, storage_align);
                        let start = offset;
                        offset += storage_size;
                        (start, 0, usize::from(bit_width))
                    };
                max_align = max_align.max(storage_align);
                fields.push(ArcanaCabiBindingLayoutField {
                    name: parsed.name,
                    ty: parsed.ty,
                    offset: storage_offset,
                    bit_width: Some(bit_width),
                    bit_offset: Some(
                        u16::try_from(next_bit_offset)
                            .map_err(|_| format!("bitfield offset overflow on `{}`", decl.name))?,
                    ),
                });
                let next = next_bit_offset + usize::from(bit_width);
                active_bitfield = Some((scalar, storage_offset, next, total_used_bits));
                if total_used_bits >= storage_bits {
                    active_bitfield = None;
                }
                continue;
            }
            active_bitfield = None;
            let (field_size, field_align) = self.raw_type_size_align(&parsed.ty)?;
            offset = align_up(offset, field_align);
            fields.push(ArcanaCabiBindingLayoutField {
                name: parsed.name,
                ty: parsed.ty,
                offset,
                bit_width: None,
                bit_offset: None,
            });
            offset += field_size;
            max_align = max_align.max(field_align);
        }
        Ok(ArcanaCabiBindingLayout {
            layout_id: binding_layout_id(&decl.module_id, &decl.name),
            size: align_up(offset, max_align),
            align: max_align,
            kind: ArcanaCabiBindingLayoutKind::Struct { fields },
        })
    }

    fn build_union_layout(
        &mut self,
        decl: &IrShackleDecl,
    ) -> Result<ArcanaCabiBindingLayout, String> {
        let mut fields = Vec::new();
        let mut size = 0usize;
        let mut align = 1usize;
        for line in &decl.body_entries {
            let parsed = parse_shackle_struct_field(line, &decl.module_id, self)?;
            if parsed.bit_width.is_some() {
                return Err(format!(
                    "shackle union `{}` does not support bitfields in the raw binding substrate",
                    decl.name
                ));
            }
            let (field_size, field_align) = self.raw_type_size_align(&parsed.ty)?;
            size = size.max(field_size);
            align = align.max(field_align);
            fields.push(ArcanaCabiBindingLayoutField {
                name: parsed.name,
                ty: parsed.ty,
                offset: 0,
                bit_width: None,
                bit_offset: None,
            });
        }
        Ok(ArcanaCabiBindingLayout {
            layout_id: binding_layout_id(&decl.module_id, &decl.name),
            size: align_up(size, align),
            align,
            kind: ArcanaCabiBindingLayoutKind::Union { fields },
        })
    }

    fn build_callback_layout(
        &mut self,
        decl: &IrShackleDecl,
    ) -> Result<ArcanaCabiBindingLayout, String> {
        let params = decl
            .params
            .iter()
            .map(|param| parse_shackle_ir_raw_type(&param.ty, &decl.module_id, self))
            .collect::<Result<Vec<_>, _>>()?;
        let return_type = decl
            .return_type
            .as_ref()
            .map(|ty| parse_shackle_ir_raw_type(ty, &decl.module_id, self))
            .transpose()?
            .unwrap_or(ArcanaCabiBindingRawType::Void);
        Ok(ArcanaCabiBindingLayout {
            layout_id: binding_layout_id(&decl.module_id, &decl.name),
            size: std::mem::size_of::<usize>(),
            align: std::mem::size_of::<usize>(),
            kind: ArcanaCabiBindingLayoutKind::Callback {
                abi: "system".to_string(),
                params,
                return_type,
            },
        })
    }

    fn build_flags_layout(
        &mut self,
        decl: &IrShackleDecl,
    ) -> Result<ArcanaCabiBindingLayout, String> {
        let layout_id = binding_layout_id(&decl.module_id, &decl.name);
        let binding = decl.binding.as_deref().ok_or_else(|| {
            format!("shackle flags `{layout_id}` is missing a repr binding target")
        })?;
        let Some(repr) = ArcanaCabiBindingScalarType::parse(binding) else {
            return Err(format!(
                "shackle flags `{layout_id}` repr `{binding}` must be a scalar integer type"
            ));
        };
        Ok(ArcanaCabiBindingLayout {
            layout_id,
            size: repr.size_bytes(),
            align: repr.align_bytes(),
            kind: ArcanaCabiBindingLayoutKind::Flags { repr },
        })
    }

    fn raw_type_size_align(
        &mut self,
        ty: &ArcanaCabiBindingRawType,
    ) -> Result<(usize, usize), String> {
        match ty {
            ArcanaCabiBindingRawType::Void => Ok((0, 1)),
            ArcanaCabiBindingRawType::Scalar(scalar) => {
                Ok((scalar.size_bytes(), scalar.align_bytes()))
            }
            ArcanaCabiBindingRawType::Pointer { .. }
            | ArcanaCabiBindingRawType::FunctionPointer { .. } => {
                let size = std::mem::size_of::<usize>();
                Ok((size, size))
            }
            ArcanaCabiBindingRawType::Named(layout_id) => {
                self.build(layout_id)?;
                let layout = self.built.get(layout_id).ok_or_else(|| {
                    format!("missing referenced raw binding layout `{layout_id}`")
                })?;
                Ok((layout.size, layout.align))
            }
        }
    }
}

fn binding_layout_id_for_decl(decl: &IrShackleDecl) -> Option<String> {
    match decl.kind.as_str() {
        "type" | "struct" | "union" | "callback" => {
            Some(binding_layout_id(&decl.module_id, &decl.name))
        }
        "flags" if decl.binding.is_some() => Some(binding_layout_id(&decl.module_id, &decl.name)),
        _ => None,
    }
}

fn binding_layout_id(module_id: &str, name: &str) -> String {
    format!("{module_id}.{name}")
}

fn companion_vtable_layout_id(module_id: &str, name: &str) -> String {
    binding_layout_id(module_id, &format!("{name}VTable"))
}

fn align_up(value: usize, align: usize) -> usize {
    if align <= 1 {
        value
    } else {
        (value + (align - 1)) & !(align - 1)
    }
}

fn split_signature_params(text: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut square_depth = 0usize;
    let mut paren_depth = 0usize;
    for ch in text.chars() {
        match ch {
            '[' => {
                square_depth += 1;
                current.push(ch);
            }
            ']' => {
                square_depth = square_depth.saturating_sub(1);
                current.push(ch);
            }
            '(' => {
                paren_depth += 1;
                current.push(ch);
            }
            ')' => {
                paren_depth = paren_depth.saturating_sub(1);
                current.push(ch);
            }
            ',' if square_depth == 0 && paren_depth == 0 => {
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    parts.push(trimmed.to_string());
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        parts.push(trimmed.to_string());
    }
    parts
}

fn split_signature_param_section(text: &str) -> Option<(String, String)> {
    let mut nested_paren_depth = 0usize;
    for (index, ch) in text.char_indices() {
        match ch {
            '(' => nested_paren_depth += 1,
            ')' => {
                if nested_paren_depth == 0 {
                    return Some((text[..index].to_string(), text[index + 1..].to_string()));
                }
                nested_paren_depth -= 1;
            }
            _ => {}
        }
    }
    None
}

fn sanitize_name(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "_".to_string()
    } else if out.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        format!("_{out}")
    } else {
        out
    }
}

fn parse_shackle_import_target(binding: &str) -> Result<IrShackleImportTarget, String> {
    let (library, symbol) = binding.split_once('.').ok_or_else(|| {
        format!("shackle import binding `{binding}` must be `<library>.<symbol>`")
    })?;
    if library.trim().is_empty() || symbol.trim().is_empty() {
        return Err(format!(
            "shackle import binding `{binding}` must use non-empty library and symbol names"
        ));
    }
    Ok(IrShackleImportTarget {
        library: library.trim().to_string(),
        symbol: symbol.trim().to_string(),
        abi: "system".to_string(),
    })
}

fn parse_shackle_struct_field(
    line: &str,
    module_id: &str,
    builder: &mut IrBindingLayoutBuilder<'_>,
) -> Result<ParsedShackleField, String> {
    let trimmed = line.trim().trim_end_matches(',');
    let (name, ty_text) = trimmed
        .split_once(':')
        .ok_or_else(|| format!("malformed shackle struct field `{trimmed}`"))?;
    let name = sanitize_name(name.trim());
    let ty_text = ty_text.trim();
    if let Some((base_text, width_text)) = ty_text.rsplit_once(" bits ") {
        let raw = parse_shackle_raw_type(base_text.trim(), module_id, builder)?;
        let bit_width = width_text
            .trim()
            .parse::<u16>()
            .map_err(|err| format!("invalid shackle bitfield width `{width_text}`: {err}"))?;
        return Ok(ParsedShackleField {
            name,
            scalar: raw_scalar_from_raw_type(&raw),
            ty: raw,
            bit_width: Some(bit_width),
        });
    }
    let raw = parse_shackle_raw_type(ty_text, module_id, builder)?;
    Ok(ParsedShackleField {
        name,
        scalar: raw_scalar_from_raw_type(&raw),
        ty: raw,
        bit_width: None,
    })
}

fn raw_scalar_from_raw_type(ty: &ArcanaCabiBindingRawType) -> Option<ArcanaCabiBindingScalarType> {
    match ty {
        ArcanaCabiBindingRawType::Scalar(scalar) => Some(*scalar),
        _ => None,
    }
}

fn parse_fixed_array_type_expr(
    text: &str,
    module_id: &str,
    builder: &mut IrBindingLayoutBuilder<'_>,
) -> Result<Option<(ArcanaCabiBindingRawType, usize)>, String> {
    let trimmed = text.trim();
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return Ok(None);
    }
    let inner = &trimmed[1..trimmed.len() - 1];
    let Some((element_text, len_text)) = inner.rsplit_once(';') else {
        return Ok(None);
    };
    let len = len_text
        .trim()
        .parse::<usize>()
        .map_err(|err| format!("invalid fixed array length `{}`: {err}", len_text.trim()))?;
    let element_type = parse_shackle_raw_type(element_text.trim(), module_id, builder)?;
    Ok(Some((element_type, len)))
}

fn parse_shackle_ir_raw_type(
    ty: &IrRoutineType,
    module_id: &str,
    builder: &mut IrBindingLayoutBuilder<'_>,
) -> Result<ArcanaCabiBindingRawType, String> {
    match &ty.kind {
        IrRoutineTypeKind::Path(path) => {
            let rendered = ty.render();
            if let Some(scalar) =
                ArcanaCabiBindingScalarType::parse(path.root_name().unwrap_or(&rendered))
            {
                return Ok(ArcanaCabiBindingRawType::Scalar(scalar));
            }
            Ok(ArcanaCabiBindingRawType::Named(
                resolve_shackle_named_layout_id(module_id, &rendered, builder),
            ))
        }
        IrRoutineTypeKind::Apply { .. }
        | IrRoutineTypeKind::Tuple(_)
        | IrRoutineTypeKind::Ref { .. }
        | IrRoutineTypeKind::Projection(_) => Err(format!(
            "unsupported raw shackle signature type `{}`",
            ty.render()
        )),
    }
}

fn parse_shackle_raw_type(
    text: &str,
    module_id: &str,
    builder: &mut IrBindingLayoutBuilder<'_>,
) -> Result<ArcanaCabiBindingRawType, String> {
    let trimmed = text.trim();
    if trimmed == "c_void" || trimmed == "()" {
        return Ok(ArcanaCabiBindingRawType::Void);
    }
    if let Some(scalar) = ArcanaCabiBindingScalarType::parse(trimmed) {
        return Ok(ArcanaCabiBindingRawType::Scalar(scalar));
    }
    if let Some(rest) = trimmed.strip_prefix("*mut ") {
        return Ok(ArcanaCabiBindingRawType::Pointer {
            mutable: true,
            inner: Box::new(parse_shackle_raw_type(rest.trim(), module_id, builder)?),
        });
    }
    if let Some(rest) = trimmed.strip_prefix("*const ") {
        return Ok(ArcanaCabiBindingRawType::Pointer {
            mutable: false,
            inner: Box::new(parse_shackle_raw_type(rest.trim(), module_id, builder)?),
        });
    }
    if let Some(function_pointer) = parse_shackle_function_pointer(trimmed, module_id, builder)? {
        return Ok(function_pointer);
    }
    Ok(ArcanaCabiBindingRawType::Named(
        resolve_shackle_named_layout_id(module_id, trimmed, builder),
    ))
}

fn parse_shackle_function_pointer(
    text: &str,
    module_id: &str,
    builder: &mut IrBindingLayoutBuilder<'_>,
) -> Result<Option<ArcanaCabiBindingRawType>, String> {
    let (nullable, inner) = if text.starts_with("Option<") && text.ends_with('>') {
        (true, &text["Option<".len()..text.len() - 1])
    } else {
        (false, text)
    };
    let inner = inner.trim();
    let inner = inner.strip_prefix("unsafe ").unwrap_or(inner).trim();
    let Some(after_extern) = inner.strip_prefix("extern ") else {
        return Ok(None);
    };
    let Some((abi_text, after_abi)) = after_extern.split_once(" fn(") else {
        return Ok(None);
    };
    let Some((params_text, return_text)) = split_signature_param_section(after_abi) else {
        return Err(format!("malformed shackle function pointer `{text}`"));
    };
    let params = split_signature_params(&params_text)
        .into_iter()
        .map(|param_text| parse_shackle_raw_type(&param_text, module_id, builder))
        .collect::<Result<Vec<_>, _>>()?;
    let return_type = if return_text.trim().is_empty() {
        ArcanaCabiBindingRawType::Void
    } else {
        let return_text = return_text
            .trim()
            .strip_prefix("->")
            .map(str::trim)
            .unwrap_or("");
        if return_text.is_empty() {
            ArcanaCabiBindingRawType::Void
        } else {
            parse_shackle_raw_type(return_text, module_id, builder)?
        }
    };
    Ok(Some(ArcanaCabiBindingRawType::FunctionPointer {
        abi: abi_text.trim().trim_matches('"').to_string(),
        nullable,
        params,
        return_type: Box::new(return_type),
    }))
}

fn resolve_shackle_named_layout_id(
    module_id: &str,
    name: &str,
    builder: &IrBindingLayoutBuilder<'_>,
) -> String {
    if name.contains('.') {
        return name.to_string();
    }
    let local = binding_layout_id(module_id, name);
    if builder.decls_by_id.contains_key(&local) {
        return local;
    }
    name.to_string()
}

#[cfg(test)]
mod tests {
    use super::{
        ExecBindLineKind, ExecExpr, ExecStmt, IrModule, lower_hir, lower_package,
        lower_workspace_package, lower_workspace_package_with_resolution,
    };
    use arcana_cabi::{
        ArcanaCabiBindingLayoutKind, ArcanaCabiBindingRawType, ArcanaCabiBindingScalarType,
    };
    use arcana_hir::{
        HirModule, build_package_layout, build_package_summary, build_workspace_package,
        build_workspace_package_with_dep_packages, build_workspace_summary, lower_module_text,
        resolve_workspace,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::Path;

    #[test]
    fn lower_hir_preserves_counts() {
        let hir = HirModule {
            symbol_count: 2,
            item_count: 7,
        };
        let ir: IrModule = lower_hir(&hir);
        assert_eq!(ir.symbol_count, 2);
        assert_eq!(ir.item_count, 7);
    }

    #[test]
    fn lower_package_preserves_public_surface_rows() {
        let summary = build_package_summary(
            "winspell",
            vec![
                lower_module_text(
                    "winspell",
                    "reexport winspell.window\nexport fn open() -> Int:\n    return 0\n",
                )
                .expect("root module should lower"),
                lower_module_text(
                    "winspell.window",
                    "import std.canvas\nexport record Window:\n    title: Text\n",
                )
                .expect("nested module should lower"),
            ],
        );

        let ir = lower_package(&summary);
        assert_eq!(ir.package_name, "winspell");
        assert_eq!(ir.root_module_id, "winspell");
        assert_eq!(ir.module_count(), 2);
        assert_eq!(ir.dependency_edge_count, 2);
        assert_eq!(
            ir.exported_surface_rows,
            vec![
                "module=winspell.window:export:record:record Window:\\ntitle: Text".to_string(),
                "module=winspell:export:fn:fn open() -> Int:".to_string(),
                "module=winspell:reexport:winspell.window".to_string(),
            ]
        );
        assert!(ir.runtime_requirements.is_empty());
        assert!(ir.entrypoints.is_empty());
        assert_eq!(ir.routines.len(), 1);
        assert_eq!(ir.routines[0].symbol_name, "open");
        assert!(ir.routines[0].params.is_empty());
        assert_eq!(
            ir.routines[0].statements,
            vec![ExecStmt::ReturnValue {
                value: ExecExpr::Int(0),
            }]
        );
        assert!(
            ir.dependency_rows
                .iter()
                .any(|row| row.contains("std.canvas"))
        );
    }

    #[test]
    fn lower_workspace_package_preserves_direct_deps() {
        let summary = build_package_summary(
            "desktop",
            vec![
                lower_module_text("desktop", "export fn main() -> Int:\n    return 0\n")
                    .expect("root module should lower"),
            ],
        );
        let layout = build_package_layout(
            &summary,
            BTreeMap::from([(
                "desktop".to_string(),
                Path::new("C:/repo/desktop/src/shelf.arc").to_path_buf(),
            )]),
            BTreeMap::new(),
        )
        .expect("layout should build");
        let workspace = build_workspace_package(
            "desktop".to_string(),
            Path::new("C:/repo/desktop").to_path_buf(),
            BTreeSet::from(["core".to_string(), "std".to_string()]),
            summary,
            layout,
        )
        .expect("workspace should build");

        let ir = lower_workspace_package(&workspace);
        assert_eq!(ir.direct_deps, vec!["core".to_string(), "std".to_string()]);
    }

    #[test]
    fn lower_package_includes_impl_methods_as_routines() {
        let summary = build_package_summary(
            "records",
            vec![
                lower_module_text(
                    "records",
                    "record Counter:\n    value: Int\nimpl Counter:\n    fn double(read self: Counter) -> Int:\n        return self.value * 2\nfn main() -> Int:\n    return 0\n",
                )
                .expect("module should lower"),
            ],
        );

        let ir = lower_package(&summary);
        assert!(
            ir.routines
                .iter()
                .any(|routine| routine.module_id == "records" && routine.symbol_name == "double"),
            "expected impl method to be lowered into routine rows"
        );
    }

    #[test]
    fn lower_workspace_package_with_resolution_emits_resolved_bare_method_paths() {
        let std_summary = build_package_summary(
            "std",
            vec![
                lower_module_text(
                    "std.collections.list",
                    "impl List[T]:\n    fn len(read self: List[T]) -> Int:\n        return 0\n",
                )
                .expect("std module should lower"),
            ],
        );
        let std_layout = build_package_layout(
            &std_summary,
            BTreeMap::from([(
                "std.collections.list".to_string(),
                Path::new("C:/repo/std/src/collections/list.arc").to_path_buf(),
            )]),
            BTreeMap::new(),
        )
        .expect("std layout should build");
        let std_workspace = build_workspace_package(
            "std".to_string(),
            Path::new("C:/repo/std").to_path_buf(),
            BTreeSet::new(),
            std_summary,
            std_layout,
        )
        .expect("std workspace should build");

        let app_summary = build_package_summary(
            "app",
            vec![lower_module_text(
                "app",
                "import std.collections.list\nfn main() -> Int:\n    let xs = [1]\n    return xs :: :: len\n",
            )
            .expect("app module should lower")],
        );
        let app_layout = build_package_layout(
            &app_summary,
            BTreeMap::from([(
                "app".to_string(),
                Path::new("C:/repo/app/src/shelf.arc").to_path_buf(),
            )]),
            BTreeMap::new(),
        )
        .expect("app layout should build");
        let app_workspace = build_workspace_package(
            "app".to_string(),
            Path::new("C:/repo/app").to_path_buf(),
            BTreeSet::from(["std".to_string()]),
            app_summary,
            app_layout,
        )
        .expect("app workspace should build");

        let workspace = build_workspace_summary(vec![std_workspace, app_workspace])
            .expect("workspace should build");
        let resolved = resolve_workspace(&workspace).expect("workspace should resolve");
        let package = workspace.package("app").expect("app package should exist");

        let ir = lower_workspace_package_with_resolution(&workspace, &resolved, package)
            .expect("workspace lowering should succeed");
        let main = ir
            .routines
            .iter()
            .find(|routine| routine.symbol_name == "main")
            .expect("main routine should exist");
        let lowered = format!("{:?}", main.statements);
        assert!(
            lowered
                .contains("resolved_callable: Some([\"std\", \"collections\", \"list\", \"len\"])"),
            "expected resolved bare-method callable path in lowered statements: {lowered}",
        );
        assert!(
            lowered.contains("resolved_routine: Some(\"std.collections.list#impl-0-method-0\")"),
            "expected resolved bare-method routine identity in lowered statements: {lowered}",
        );
    }

    #[test]
    fn lower_workspace_package_with_resolution_resolves_bare_methods_on_generic_enum_values() {
        let std_summary = build_package_summary(
            "std",
            vec![
                lower_module_text(
                    "std.result",
                    concat!(
                        "export enum Result[T, E]:\n",
                        "    Ok(T)\n",
                        "    Err(E)\n",
                        "impl[T, E] Result[T, E]:\n",
                        "    fn is_ok(read self: Result[T, E]) -> Bool:\n",
                        "        return true\n",
                        "    fn unwrap_or(read self: Result[T, E], take fallback: T) -> T:\n",
                        "        return fallback\n",
                    ),
                )
                .expect("std result module should lower"),
            ],
        );
        let std_layout = build_package_layout(
            &std_summary,
            BTreeMap::from([(
                "std.result".to_string(),
                Path::new("C:/repo/std/src/result.arc").to_path_buf(),
            )]),
            BTreeMap::new(),
        )
        .expect("std layout should build");
        let std_workspace = build_workspace_package(
            "std".to_string(),
            Path::new("C:/repo/std").to_path_buf(),
            BTreeSet::new(),
            std_summary,
            std_layout,
        )
        .expect("std workspace should build");

        let app_summary = build_package_summary(
            "app",
            vec![
                lower_module_text(
                    "app",
                    concat!(
                        "import std.result\n",
                        "use std.result.Result\n",
                        "fn main() -> Int:\n",
                        "    let ok = Result.Ok[Int, Str] :: 7 :: call\n",
                        "    let check = ok :: :: is_ok\n",
                        "    return ok :: 13 :: unwrap_or\n",
                    ),
                )
                .expect("app module should lower"),
            ],
        );
        let app_layout = build_package_layout(
            &app_summary,
            BTreeMap::from([(
                "app".to_string(),
                Path::new("C:/repo/app/src/shelf.arc").to_path_buf(),
            )]),
            BTreeMap::new(),
        )
        .expect("app layout should build");
        let app_workspace = build_workspace_package(
            "app".to_string(),
            Path::new("C:/repo/app").to_path_buf(),
            BTreeSet::from(["std".to_string()]),
            app_summary,
            app_layout,
        )
        .expect("app workspace should build");

        let workspace = build_workspace_summary(vec![std_workspace, app_workspace])
            .expect("workspace should build");
        let resolved = resolve_workspace(&workspace).expect("workspace should resolve");
        let package = workspace.package("app").expect("app package should exist");

        let ir = lower_workspace_package_with_resolution(&workspace, &resolved, package)
            .expect("workspace lowering should succeed");
        let main = ir
            .routines
            .iter()
            .find(|routine| routine.symbol_name == "main")
            .expect("main routine should exist");
        let lowered = format!("{:?}", main.statements);
        assert!(
            lowered.contains("qualifier: \"is_ok\"")
                && lowered.contains("resolved_callable: Some([\"std\", \"result\", \"is_ok\"])")
                && lowered.contains("resolved_routine: Some(\"std.result#impl-0-method-0\")"),
            "expected resolved Result.is_ok bare method in lowered statements: {lowered}",
        );
        assert!(
            lowered.contains("qualifier: \"unwrap_or\"")
                && lowered
                    .contains("resolved_callable: Some([\"std\", \"result\", \"unwrap_or\"])")
                && lowered.contains("resolved_routine: Some(\"std.result#impl-0-method-1\")"),
            "expected resolved Result.unwrap_or bare method in lowered statements: {lowered}",
        );
    }

    #[test]
    fn lower_workspace_package_with_resolution_resolves_bare_methods_on_spawn_handles() {
        let std_summary = build_package_summary(
            "std",
            vec![
                lower_module_text(
                    "std.concurrent",
                    concat!(
                        "impl[T] Task[T]:\n",
                        "    fn done(read self: Task[T]) -> Bool:\n",
                        "        return false\n",
                        "    fn join(read self: Task[T]) -> T:\n",
                        "        return 0\n",
                        "impl[T] Thread[T]:\n",
                        "    fn done(read self: Thread[T]) -> Bool:\n",
                        "        return false\n",
                        "    fn join(read self: Thread[T]) -> T:\n",
                        "        return 0\n",
                    ),
                )
                .expect("std concurrent module should lower"),
            ],
        );
        let std_layout = build_package_layout(
            &std_summary,
            BTreeMap::from([(
                "std.concurrent".to_string(),
                Path::new("C:/repo/std/src/concurrent.arc").to_path_buf(),
            )]),
            BTreeMap::new(),
        )
        .expect("std layout should build");
        let std_workspace = build_workspace_package(
            "std".to_string(),
            Path::new("C:/repo/std").to_path_buf(),
            BTreeSet::new(),
            std_summary,
            std_layout,
        )
        .expect("std workspace should build");

        let app_summary = build_package_summary(
            "app",
            vec![
                lower_module_text(
                    "app",
                    concat!(
                        "fn worker() -> Int:\n",
                        "    return 1\n",
                        "fn helper() -> Int:\n",
                        "    return 2\n",
                        "fn main() -> Int:\n",
                        "    let task = weave worker :: :: call\n",
                        "    let thread = split helper :: :: call\n",
                        "    if task :: :: done:\n",
                        "        return thread :: :: join\n",
                        "    return task :: :: join\n",
                    ),
                )
                .expect("app module should lower"),
            ],
        );
        let app_layout = build_package_layout(
            &app_summary,
            BTreeMap::from([(
                "app".to_string(),
                Path::new("C:/repo/app/src/shelf.arc").to_path_buf(),
            )]),
            BTreeMap::new(),
        )
        .expect("app layout should build");
        let app_workspace = build_workspace_package(
            "app".to_string(),
            Path::new("C:/repo/app").to_path_buf(),
            BTreeSet::from(["std".to_string()]),
            app_summary,
            app_layout,
        )
        .expect("app workspace should build");

        let workspace = build_workspace_summary(vec![std_workspace, app_workspace])
            .expect("workspace should build");
        let resolved = resolve_workspace(&workspace).expect("workspace should resolve");
        let package = workspace.package("app").expect("app package should exist");

        let ir = lower_workspace_package_with_resolution(&workspace, &resolved, package)
            .expect("workspace lowering should succeed");
        let main = ir
            .routines
            .iter()
            .find(|routine| routine.symbol_name == "main")
            .expect("main routine should exist");
        let lowered = format!("{:?}", main.statements);
        assert!(
            lowered.contains("qualifier: \"done\"")
                && lowered.contains("resolved_callable: Some([\"std\", \"concurrent\", \"done\"])"),
            "expected resolved Task.done bare method on spawned handle: {lowered}",
        );
        assert!(
            lowered.contains("qualifier: \"join\"")
                && lowered.contains("resolved_callable: Some([\"std\", \"concurrent\", \"join\"])")
                && (lowered.contains("resolved_routine: Some(\"std.concurrent#impl-1-method-1\")")
                    || lowered
                        .contains("resolved_routine: Some(\"std.concurrent#impl-0-method-1\")")),
            "expected resolved join bare method on spawned handle: {lowered}",
        );
    }

    #[test]
    fn lower_workspace_package_with_resolution_rejects_ambiguous_concrete_bare_methods() {
        let app_summary = build_package_summary(
            "app",
            vec![
                lower_module_text(
                    "app",
                    concat!(
                        "record Counter:\n",
                        "    value: Int\n",
                        "impl Counter:\n",
                        "    fn tap(read self: Counter) -> Int:\n",
                        "        return self.value + 1\n",
                        "impl Counter:\n",
                        "    fn tap(read self: Counter) -> Int:\n",
                        "        return self.value + 2\n",
                        "fn main() -> Int:\n",
                        "    let counter = Counter :: value = 1 :: call\n",
                        "    return counter :: :: tap\n",
                    ),
                )
                .expect("module should lower"),
            ],
        );
        let app_layout = build_package_layout(
            &app_summary,
            BTreeMap::from([(
                "app".to_string(),
                Path::new("C:/repo/app/src/shelf.arc").to_path_buf(),
            )]),
            BTreeMap::new(),
        )
        .expect("app layout should build");
        let app_workspace = build_workspace_package(
            "app".to_string(),
            Path::new("C:/repo/app").to_path_buf(),
            BTreeSet::new(),
            app_summary,
            app_layout,
        )
        .expect("app workspace should build");

        let workspace =
            build_workspace_summary(vec![app_workspace]).expect("workspace should build");
        let resolved = resolve_workspace(&workspace).expect("workspace should resolve");
        let package = workspace.package("app").expect("app package should exist");

        let err = lower_workspace_package_with_resolution(&workspace, &resolved, package)
            .expect_err("ambiguous concrete bare method should fail lowering");
        assert!(
            err.contains("bare-method qualifier `tap` on `app.Counter` is ambiguous"),
            "{err}"
        );
    }

    #[test]
    fn lower_workspace_package_with_resolution_resolves_bare_methods_on_bind_survivors() {
        let app_summary = build_package_summary(
            "app",
            vec![
                lower_module_text(
                    "app",
                    concat!(
                        "record Counter:\n",
                        "    value: Int\n",
                        "impl Counter:\n",
                        "    fn tap(read self: Counter) -> Int:\n",
                        "        return self.value\n",
                        "enum Result[T, E]:\n",
                        "    Ok(T)\n",
                        "    Err(E)\n",
                        "fn main() -> Int:\n",
                        "    bind -return 0\n",
                        "        let counter = Result.Ok[Counter, Str] :: (Counter :: value = 1 :: call) :: call\n",
                        "        require (counter :: :: tap) == 1\n",
                        "    return counter :: :: tap\n",
                    ),
                )
                .expect("module should lower"),
            ],
        );
        let app_layout = build_package_layout(
            &app_summary,
            BTreeMap::from([(
                "app".to_string(),
                Path::new("C:/repo/app/src/shelf.arc").to_path_buf(),
            )]),
            BTreeMap::new(),
        )
        .expect("app layout should build");
        let app_workspace = build_workspace_package(
            "app".to_string(),
            Path::new("C:/repo/app").to_path_buf(),
            BTreeSet::new(),
            app_summary,
            app_layout,
        )
        .expect("app workspace should build");

        let workspace =
            build_workspace_summary(vec![app_workspace]).expect("workspace should build");
        let resolved = resolve_workspace(&workspace).expect("workspace should resolve");
        let package = workspace.package("app").expect("app package should exist");

        let lowered = lower_workspace_package_with_resolution(&workspace, &resolved, package)
            .expect("workspace package should lower");
        let main = lowered
            .routines
            .iter()
            .find(|routine| routine.symbol_name == "main")
            .expect("main routine should exist");
        let ExecStmt::Bind { lines, .. } = &main.statements[0] else {
            panic!("expected bind region");
        };
        let ExecBindLineKind::Require { expr } = &lines[1].kind else {
            panic!("expected require line");
        };
        let ExecExpr::Binary { left, .. } = expr else {
            panic!("expected binary require expression");
        };
        let ExecExpr::Phrase {
            resolved_routine, ..
        } = left.as_ref()
        else {
            panic!("expected bare-method phrase inside bind require");
        };
        assert!(
            resolved_routine.is_some(),
            "bind survivor should resolve bare method"
        );
        let ExecStmt::ReturnValue { value } = &main.statements[1] else {
            panic!("expected return statement");
        };
        let ExecExpr::Phrase {
            resolved_routine, ..
        } = value
        else {
            panic!("expected bare-method return phrase");
        };
        assert!(
            resolved_routine.is_some(),
            "post-bind survivor should resolve bare method"
        );
    }

    #[test]
    fn lower_workspace_package_with_resolution_resolves_bare_methods_on_construct_deliver_survivors()
     {
        let app_summary = build_package_summary(
            "app",
            vec![
                lower_module_text(
                    "app",
                    concat!(
                        "record Counter:\n",
                        "    value: Int\n",
                        "impl Counter:\n",
                        "    fn tap(read self: Counter) -> Int:\n",
                        "        return self.value\n",
                        "fn main() -> Int:\n",
                        "    construct deliver Counter -> counter -return 0\n",
                        "        value = 1\n",
                        "    return counter :: :: tap\n",
                    ),
                )
                .expect("module should lower"),
            ],
        );
        let app_layout = build_package_layout(
            &app_summary,
            BTreeMap::from([(
                "app".to_string(),
                Path::new("C:/repo/app/src/shelf.arc").to_path_buf(),
            )]),
            BTreeMap::new(),
        )
        .expect("app layout should build");
        let app_workspace = build_workspace_package(
            "app".to_string(),
            Path::new("C:/repo/app").to_path_buf(),
            BTreeSet::new(),
            app_summary,
            app_layout,
        )
        .expect("app workspace should build");

        let workspace =
            build_workspace_summary(vec![app_workspace]).expect("workspace should build");
        let resolved = resolve_workspace(&workspace).expect("workspace should resolve");
        let package = workspace.package("app").expect("app package should exist");

        let lowered = lower_workspace_package_with_resolution(&workspace, &resolved, package)
            .expect("workspace package should lower");
        let main = lowered
            .routines
            .iter()
            .find(|routine| routine.symbol_name == "main")
            .expect("main routine should exist");
        let ExecStmt::ReturnValue { value } = &main.statements[1] else {
            panic!("expected return statement");
        };
        let ExecExpr::Phrase {
            resolved_routine, ..
        } = value
        else {
            panic!("expected bare-method return phrase");
        };
        assert!(
            resolved_routine.is_some(),
            "construct deliver survivor should resolve bare method"
        );
    }

    #[test]
    fn lower_workspace_package_with_resolution_collects_record_copied_fields() {
        let app_summary = build_package_summary(
            "app",
            vec![
                lower_module_text(
                    "app",
                    concat!(
                        "record Seed:\n",
                        "    title: Str\n",
                        "    count: Int\n",
                        "record Widget:\n",
                        "    title: Str\n",
                        "    count: Int\n",
                        "    ready: Bool\n",
                        "fn main() -> Int:\n",
                        "    let base = Seed :: title = \"seed\", count = 4 :: call\n",
                        "    let built = record yield Widget from base -return 0\n",
                        "        ready = true\n",
                        "    return built.count\n",
                    ),
                )
                .expect("module should lower"),
            ],
        );
        let app_layout = build_package_layout(
            &app_summary,
            BTreeMap::from([(
                "app".to_string(),
                Path::new("C:/repo/app/src/shelf.arc").to_path_buf(),
            )]),
            BTreeMap::new(),
        )
        .expect("app layout should build");
        let app_workspace = build_workspace_package(
            "app".to_string(),
            Path::new("C:/repo/app").to_path_buf(),
            BTreeSet::new(),
            app_summary,
            app_layout,
        )
        .expect("app workspace should build");

        let workspace =
            build_workspace_summary(vec![app_workspace]).expect("workspace should build");
        let resolved = resolve_workspace(&workspace).expect("workspace should resolve");
        let package = workspace.package("app").expect("app package should exist");

        let lowered = lower_workspace_package_with_resolution(&workspace, &resolved, package)
            .expect("workspace package should lower");
        let main = lowered
            .routines
            .iter()
            .find(|routine| routine.symbol_name == "main")
            .expect("main routine should exist");
        let ExecStmt::Let { value, .. } = &main.statements[1] else {
            panic!("expected let binding for record yield");
        };
        let ExecExpr::RecordRegion(region) = value else {
            panic!("expected record region expression");
        };
        assert_eq!(
            region.copied_fields,
            vec!["count".to_string(), "title".to_string()],
            "resolved lowering should carry exact copied fields for record lift",
        );
    }

    #[test]
    fn lower_workspace_package_with_resolution_collects_record_copied_fields_from_construct_base() {
        let app_summary = build_package_summary(
            "app",
            vec![
                lower_module_text(
                    "app",
                    concat!(
                        "record Widget:\n",
                        "    ready: Bool\n",
                        "    title: Str\n",
                        "    count: Int\n",
                        "fn main() -> Int:\n",
                        "    let base = construct yield Widget -return 0\n",
                        "        ready = false\n",
                        "        title = \"seed\"\n",
                        "        count = 4\n",
                        "    let built = record yield Widget from base -return 0\n",
                        "        ready = true\n",
                        "    return 0\n",
                    ),
                )
                .expect("module should lower"),
            ],
        );
        let app_layout = build_package_layout(
            &app_summary,
            BTreeMap::from([(
                "app".to_string(),
                Path::new("C:/repo/app/src/shelf.arc").to_path_buf(),
            )]),
            BTreeMap::new(),
        )
        .expect("app layout should build");
        let app_workspace = build_workspace_package(
            "app".to_string(),
            Path::new("C:/repo/app").to_path_buf(),
            BTreeSet::new(),
            app_summary,
            app_layout,
        )
        .expect("app workspace should build");

        let workspace =
            build_workspace_summary(vec![app_workspace]).expect("workspace should build");
        let resolved = resolve_workspace(&workspace).expect("workspace should resolve");
        let package = workspace.package("app").expect("app package should exist");

        let lowered = lower_workspace_package_with_resolution(&workspace, &resolved, package)
            .expect("workspace package should lower");
        let main = lowered
            .routines
            .iter()
            .find(|routine| routine.symbol_name == "main")
            .expect("main routine should exist");
        let ExecStmt::Let { value, .. } = &main.statements[1] else {
            panic!("expected let binding for record yield");
        };
        let ExecExpr::RecordRegion(region) = value else {
            panic!("expected record region expression");
        };
        assert_eq!(
            region.copied_fields,
            vec!["count".to_string(), "title".to_string()],
            "resolved lowering should copy omitted same-type fields from construct bases",
        );
    }

    #[test]
    fn lower_workspace_package_with_resolution_canonicalizes_dependency_alias_metadata() {
        let core_summary = build_package_summary(
            "core",
            vec![
                lower_module_text("core", "export fn value() -> Int:\n    return 7\n")
                    .expect("core module should lower"),
            ],
        );
        let core_layout = build_package_layout(
            &core_summary,
            BTreeMap::from([(
                "core".to_string(),
                Path::new("C:/repo/core/src/book.arc").to_path_buf(),
            )]),
            BTreeMap::new(),
        )
        .expect("core layout should build");
        let core_workspace = build_workspace_package(
            "core".to_string(),
            Path::new("C:/repo/core").to_path_buf(),
            BTreeSet::new(),
            core_summary,
            core_layout,
        )
        .expect("core package should build");

        let app_summary = build_package_summary(
            "app",
            vec![
                lower_module_text(
                    "app",
                    "import util\nfn main() -> Int:\n    return util.value :: :: call\n",
                )
                .expect("app module should lower"),
            ],
        );
        let app_layout = build_package_layout(
            &app_summary,
            BTreeMap::from([(
                "app".to_string(),
                Path::new("C:/repo/app/src/shelf.arc").to_path_buf(),
            )]),
            BTreeMap::new(),
        )
        .expect("app layout should build");
        let app_workspace = build_workspace_package_with_dep_packages(
            "app".to_string(),
            Path::new("C:/repo/app").to_path_buf(),
            BTreeMap::from([("util".to_string(), "core".to_string())]),
            BTreeMap::from([("util".to_string(), "core".to_string())]),
            app_summary,
            app_layout,
        )
        .expect("app package should build");

        let workspace =
            build_workspace_summary(vec![app_workspace, core_workspace]).expect("workspace");
        let resolved = resolve_workspace(&workspace).expect("workspace should resolve");
        let package = workspace.package("app").expect("app package should exist");

        let ir = lower_workspace_package_with_resolution(&workspace, &resolved, package)
            .expect("workspace lowering should succeed");
        assert_eq!(ir.direct_deps, vec!["core".to_string()]);
        assert_eq!(
            ir.dependency_rows,
            vec!["source=app:import:core:".to_string()]
        );
    }

    #[test]
    fn lower_workspace_package_with_resolution_canonicalizes_lang_item_targets() {
        let app_summary = build_package_summary(
            "app",
            vec![
                lower_module_text(
                    "app",
                    concat!(
                        "import app.types\n",
                        "lang window_handle = types.Window\n",
                        "fn main() -> Int:\n",
                        "    return 0\n",
                    ),
                )
                .expect("root module should lower"),
                lower_module_text(
                    "app.types",
                    "export opaque type Window as move, boundary_unsafe\n",
                )
                .expect("types module should lower"),
            ],
        );
        let app_layout = build_package_layout(
            &app_summary,
            BTreeMap::from([
                (
                    "app".to_string(),
                    Path::new("C:/repo/app/src/shelf.arc").to_path_buf(),
                ),
                (
                    "app.types".to_string(),
                    Path::new("C:/repo/app/src/types.arc").to_path_buf(),
                ),
            ]),
            BTreeMap::new(),
        )
        .expect("app layout should build");
        let app_workspace = build_workspace_package(
            "app".to_string(),
            Path::new("C:/repo/app").to_path_buf(),
            BTreeSet::new(),
            app_summary,
            app_layout,
        )
        .expect("app workspace should build");

        let workspace =
            build_workspace_summary(vec![app_workspace]).expect("workspace should build");
        let resolved = resolve_workspace(&workspace).expect("workspace should resolve");
        let package = workspace.package("app").expect("app package should exist");

        let ir = lower_workspace_package_with_resolution(&workspace, &resolved, package)
            .expect("workspace lowering should succeed");
        let root_module = ir
            .modules
            .iter()
            .find(|module| module.module_id == "app")
            .expect("root module should exist");
        assert_eq!(
            root_module.lang_item_rows,
            vec!["module=app:lang:window_handle:app.types.Window".to_string()]
        );
    }

    #[test]
    fn lower_workspace_package_with_resolution_canonicalizes_cleanup_footer_handler_paths() {
        let app_summary = build_package_summary(
            "app",
            vec![
                lower_module_text(
                    "app",
                    concat!(
                        "import app.handlers\n",
                        "lang cleanup_contract = handlers.Cleanup\n",
                        "fn main() -> Int:\n",
                        "    let value = 1\n",
                        "    return 0\n",
                        "-cleanup[target = value, handler = handlers.cleanup]\n",
                    ),
                )
                .expect("root module should lower"),
                lower_module_text(
                    "app.handlers",
                    concat!(
                        "export enum Result[T, E]:\n",
                        "    Ok(T)\n",
                        "    Err(E)\n",
                        "export trait Cleanup[T]:\n",
                        "    fn cleanup(take self: T) -> Result[Unit, Str]\n",
                        "impl app.handlers.Cleanup[Int] for Int:\n",
                        "    fn cleanup(take self: Int) -> Result[Unit, Str]:\n",
                        "        return Result.Ok[Unit, Str] :: :: call\n",
                        "export fn cleanup(take value: Int) -> Result[Unit, Str]:\n",
                        "    return Result.Ok[Unit, Str] :: :: call\n",
                    ),
                )
                .expect("handler module should lower"),
            ],
        );
        let app_layout = build_package_layout(
            &app_summary,
            BTreeMap::from([
                (
                    "app".to_string(),
                    Path::new("C:/repo/app/src/shelf.arc").to_path_buf(),
                ),
                (
                    "app.handlers".to_string(),
                    Path::new("C:/repo/app/src/handlers.arc").to_path_buf(),
                ),
            ]),
            BTreeMap::new(),
        )
        .expect("app layout should build");
        let app_workspace = build_workspace_package(
            "app".to_string(),
            Path::new("C:/repo/app").to_path_buf(),
            BTreeSet::new(),
            app_summary,
            app_layout,
        )
        .expect("app workspace should build");

        let workspace =
            build_workspace_summary(vec![app_workspace]).expect("workspace should build");
        let resolved = resolve_workspace(&workspace).expect("workspace should resolve");
        let package = workspace.package("app").expect("app package should exist");

        let ir = lower_workspace_package_with_resolution(&workspace, &resolved, package)
            .expect("workspace lowering should succeed");
        let main = ir
            .routines
            .iter()
            .find(|routine| routine.symbol_name == "main")
            .expect("main routine should exist");

        assert_eq!(main.cleanup_footers.len(), 1);
        assert_eq!(
            main.cleanup_footers[0].handler_path,
            vec![
                "app".to_string(),
                "handlers".to_string(),
                "cleanup".to_string()
            ]
        );
    }

    #[test]
    fn lower_workspace_package_with_resolution_rejects_ambiguous_cleanup_footer_target_under_shadowing()
     {
        let app_summary = build_package_summary(
            "app",
            vec![
                lower_module_text(
                    "app",
                    concat!(
                        "import app.handlers\n",
                        "lang cleanup_contract = handlers.Cleanup\n",
                        "record Box:\n",
                        "    value: Int\n",
                        "fn main() -> Int:\n",
                        "    let x = Box :: value = 1 :: call\n",
                        "    if true:\n",
                        "        let x = Box :: value = 2 :: call\n",
                        "    return 0\n",
                        "-cleanup[target = x, handler = handlers.cleanup]\n",
                    ),
                )
                .expect("root module should lower"),
                lower_module_text(
                    "app.handlers",
                    concat!(
                        "export enum Result[T, E]:\n",
                        "    Ok(T)\n",
                        "    Err(E)\n",
                        "export trait Cleanup[T]:\n",
                        "    fn cleanup(take self: T) -> Result[Unit, Str]\n",
                        "use app.Box\n",
                        "impl app.handlers.Cleanup[Box] for Box:\n",
                        "    fn cleanup(take self: Box) -> Result[Unit, Str]:\n",
                        "        return Result.Ok[Unit, Str] :: :: call\n",
                        "export fn cleanup(take value: Box) -> Result[Unit, Str]:\n",
                        "    return Result.Ok[Unit, Str] :: :: call\n",
                    ),
                )
                .expect("handler module should lower"),
            ],
        );
        let app_layout = build_package_layout(
            &app_summary,
            BTreeMap::from([
                (
                    "app".to_string(),
                    Path::new("C:/repo/app/src/shelf.arc").to_path_buf(),
                ),
                (
                    "app.handlers".to_string(),
                    Path::new("C:/repo/app/src/handlers.arc").to_path_buf(),
                ),
            ]),
            BTreeMap::new(),
        )
        .expect("app layout should build");
        let app_workspace = build_workspace_package(
            "app".to_string(),
            Path::new("C:/repo/app").to_path_buf(),
            BTreeSet::new(),
            app_summary,
            app_layout,
        )
        .expect("app workspace should build");

        let workspace =
            build_workspace_summary(vec![app_workspace]).expect("workspace should build");
        let resolved = resolve_workspace(&workspace).expect("workspace should resolve");
        let package = workspace.package("app").expect("app package should exist");

        let err = lower_workspace_package_with_resolution(&workspace, &resolved, package)
            .expect_err("workspace lowering should reject ambiguous cleanup target");
        assert!(
            err.contains("cleanup footer target `x` is ambiguous in the owning scope"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn lower_workspace_package_with_resolution_marks_public_impl_methods_exported() {
        let core_summary = build_package_summary(
            "core",
            vec![
                lower_module_text(
                    "core",
                    concat!(
                        "export record Counter:\n",
                        "    value: Int\n",
                        "impl Counter:\n",
                        "    fn announce(read self: Counter) -> Int:\n",
                        "        return self.value\n",
                    ),
                )
                .expect("core module should lower"),
            ],
        );
        let core_layout = build_package_layout(
            &core_summary,
            BTreeMap::from([(
                "core".to_string(),
                Path::new("C:/repo/core/src/book.arc").to_path_buf(),
            )]),
            BTreeMap::new(),
        )
        .expect("core layout should build");
        let core_workspace = build_workspace_package(
            "core".to_string(),
            Path::new("C:/repo/core").to_path_buf(),
            BTreeSet::new(),
            core_summary,
            core_layout,
        )
        .expect("core workspace should build");

        let workspace =
            build_workspace_summary(vec![core_workspace]).expect("workspace should build");
        let resolved = resolve_workspace(&workspace).expect("workspace should resolve");
        let package = workspace
            .package("core")
            .expect("core package should exist");

        let ir = lower_workspace_package_with_resolution(&workspace, &resolved, package)
            .expect("workspace lowering should succeed");
        let announce = ir
            .routines
            .iter()
            .find(|routine| routine.symbol_name == "announce")
            .expect("impl method should lower");

        assert!(announce.exported);
    }

    #[test]
    fn lower_workspace_package_with_resolution_carries_shackle_decls_and_typed_callbacks() {
        let app_summary = build_package_summary(
            "app",
            vec![
                lower_module_text(
                    "app",
                    "native callback proc: app.raw.WNDPROC = app.callbacks.handle_proc\n",
                )
                .expect("app module should lower"),
                lower_module_text(
                    "app.raw",
                    "export shackle callback WNDPROC(read hwnd: Int, message: Int) -> Int\n",
                )
                .expect("raw module should lower"),
                lower_module_text(
                    "app.callbacks",
                    "fn handle_proc(read code: Int) -> Int:\n    return code\n",
                )
                .expect("callbacks module should lower"),
            ],
        );
        let app_layout = build_package_layout(
            &app_summary,
            BTreeMap::from([
                (
                    "app".to_string(),
                    Path::new("C:/repo/app/src/book.arc").to_path_buf(),
                ),
                (
                    "app.raw".to_string(),
                    Path::new("C:/repo/app/src/raw.arc").to_path_buf(),
                ),
                (
                    "app.callbacks".to_string(),
                    Path::new("C:/repo/app/src/callbacks.arc").to_path_buf(),
                ),
            ]),
            BTreeMap::from([
                ("raw".to_string(), "app.raw".to_string()),
                ("callbacks".to_string(), "app.callbacks".to_string()),
            ]),
        )
        .expect("app layout should build");
        let app_workspace = build_workspace_package(
            "app".to_string(),
            Path::new("C:/repo/app").to_path_buf(),
            BTreeSet::new(),
            app_summary,
            app_layout,
        )
        .expect("app workspace should build");

        let workspace =
            build_workspace_summary(vec![app_workspace]).expect("workspace should build");
        let resolved = resolve_workspace(&workspace).expect("workspace should resolve");
        let package = workspace.package("app").expect("app package should exist");

        let ir = lower_workspace_package_with_resolution(&workspace, &resolved, package)
            .expect("workspace lowering should succeed");
        assert_eq!(ir.shackle_decls.len(), 1);
        assert_eq!(ir.shackle_decls[0].kind, "callback");
        assert_eq!(ir.shackle_decls[0].name, "WNDPROC");
        assert_eq!(ir.native_callbacks.len(), 1);
        assert_eq!(ir.native_callbacks[0].params.len(), 2);
        assert_eq!(ir.native_callbacks[0].params[0].name, "hwnd");
        assert_eq!(ir.native_callbacks[0].params[1].name, "message");
        assert_eq!(
            ir.native_callbacks[0]
                .return_type
                .as_ref()
                .expect("typed callback return type")
                .render(),
            "Int"
        );
        assert_eq!(
            ir.native_callbacks[0]
                .callback_type
                .as_ref()
                .expect("typed callback ref")
                .render(),
            "app.raw.WNDPROC"
        );
        let callback_layout = ir.shackle_decls[0]
            .raw_layout
            .as_ref()
            .expect("callback should lower typed raw metadata");
        assert!(matches!(
            callback_layout.kind,
            ArcanaCabiBindingLayoutKind::Callback { .. }
        ));
    }

    #[test]
    fn lower_workspace_package_with_resolution_lowers_typed_shackle_raw_metadata() {
        let app_summary =
            build_package_summary(
                "app",
                vec![lower_module_text(
                "app.raw",
                concat!(
                    "export shackle struct Rect:\n",
                    "    left: I32\n",
                    "    top: I32\n",
                    "    flags: U32 bits 3\n",
                    "export shackle struct IUnknownVTable:\n",
                    "    query_interface: unsafe extern \"system\" fn(*const c_void) -> I32\n",
                    "export shackle type IUnknown = *mut c_void\n",
                    "export shackle import fn CoInitializeEx() -> I32 = ole32.CoInitializeEx\n",
                ),
            )
            .expect("raw module should lower")],
            );
        let app_layout = build_package_layout(
            &app_summary,
            BTreeMap::from([(
                "app.raw".to_string(),
                Path::new("C:/repo/app/src/raw.arc").to_path_buf(),
            )]),
            BTreeMap::new(),
        )
        .expect("app layout should build");
        let app_workspace = build_workspace_package(
            "app".to_string(),
            Path::new("C:/repo/app").to_path_buf(),
            BTreeSet::new(),
            app_summary,
            app_layout,
        )
        .expect("app workspace should build");

        let workspace =
            build_workspace_summary(vec![app_workspace]).expect("workspace should build");
        let resolved = resolve_workspace(&workspace).expect("workspace should resolve");
        let package = workspace.package("app").expect("app package should exist");

        let ir = lower_workspace_package_with_resolution(&workspace, &resolved, package)
            .expect("workspace lowering should succeed");
        let rect = ir
            .shackle_decls
            .iter()
            .find(|decl| decl.name == "Rect")
            .expect("Rect layout should lower");
        let rect_layout = rect
            .raw_layout
            .as_ref()
            .expect("Rect should carry typed raw layout metadata");
        let ArcanaCabiBindingLayoutKind::Struct { fields } = &rect_layout.kind else {
            panic!("Rect should lower as a struct layout");
        };
        assert_eq!(fields.len(), 3);
        assert_eq!(fields[2].name, "flags");
        assert_eq!(fields[2].bit_width, Some(3));
        assert_eq!(fields[2].bit_offset, Some(0));
        assert_eq!(
            fields[2].ty,
            ArcanaCabiBindingRawType::Scalar(ArcanaCabiBindingScalarType::U32)
        );

        let interface = ir
            .shackle_decls
            .iter()
            .find(|decl| decl.name == "IUnknown")
            .expect("IUnknown layout should lower");
        let interface_layout = interface
            .raw_layout
            .as_ref()
            .expect("IUnknown should carry typed raw layout metadata");
        let ArcanaCabiBindingLayoutKind::Interface {
            vtable_layout_id, ..
        } = &interface_layout.kind
        else {
            panic!("IUnknown should lower as an interface layout");
        };
        assert_eq!(vtable_layout_id.as_deref(), Some("app.raw.IUnknownVTable"));

        let import = ir
            .shackle_decls
            .iter()
            .find(|decl| decl.name == "CoInitializeEx")
            .expect("import should lower");
        let target = import
            .import_target
            .as_ref()
            .expect("import should carry typed import target metadata");
        assert_eq!(target.library, "ole32");
        assert_eq!(target.symbol, "CoInitializeEx");
        assert_eq!(target.abi, "system");
    }
}
