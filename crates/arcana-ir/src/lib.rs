#![allow(clippy::too_many_arguments)]

mod entrypoint;
mod executable;
mod routine_signature;
mod runtime_requirements;

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use arcana_hir::{
    HirAssignOp, HirAssignTarget, HirBinaryOp, HirChainConnector, HirChainIntroducer, HirChainStep,
    HirCleanupFooter, HirDirectiveKind, HirExpr, HirForewordApp, HirForewordArg,
    HirHeaderAttachment, HirLocalTypeLookup, HirMatchPattern, HirModule, HirModuleDependency,
    HirModuleSummary, HirPackageSummary, HirPath, HirPhraseArg, HirPredicate, HirResolvedModule,
    HirResolvedWorkspace, HirStatement, HirStatementKind, HirSymbol, HirSymbolBody, HirSymbolKind,
    HirType, HirTypeBindingScope, HirTypeKind, HirTypeSubstitutions, HirUnaryOp, HirWhereClause,
    HirWorkspacePackage, HirWorkspaceSummary, canonicalize_hir_type_in_module,
    current_workspace_package_for_module, hir_type_matches, impl_target_is_public_from_package,
    infer_receiver_expr_type, lookup_method_candidates_for_hir_type, lookup_symbol_path,
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
    ExecAssignOp, ExecAssignTarget, ExecAvailabilityAttachment, ExecAvailabilityKind, ExecBinaryOp,
    ExecBindLine, ExecBindLineKind, ExecChainConnector, ExecChainIntroducer, ExecChainStep,
    ExecCleanupFooter, ExecConstructContributionMode, ExecConstructDestination, ExecConstructLine,
    ExecConstructRegion, ExecDynamicDispatch, ExecExpr, ExecHeadedModifier, ExecHeaderAttachment,
    ExecMatchArm, ExecMatchPattern, ExecMemoryDetailLine, ExecMemorySpecDecl, ExecNamedBindingId,
    ExecPhraseArg, ExecPhraseQualifierKind, ExecRecycleLine, ExecRecycleLineKind, ExecStmt,
    ExecUnaryOp,
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
    pub impl_target_type: Option<IrRoutineType>,
    pub impl_trait_path: Option<Vec<String>>,
    pub availability: Vec<ExecAvailabilityAttachment>,
    pub foreword_rows: Vec<String>,
    pub cleanup_footers: Vec<ExecCleanupFooter>,
    pub statements: Vec<ExecStmt>,
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
    pub holds: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrOwnerDecl {
    pub package_id: String,
    pub module_id: String,
    pub owner_path: Vec<String>,
    pub owner_name: String,
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
    pub entrypoints: Vec<IrEntrypoint>,
    pub routines: Vec<IrRoutine>,
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
        ExecStmt::Defer(expr) => rewrite_expr_routine_keys(
            package_display_names,
            package_direct_dep_ids,
            duplicate_keys,
            routine_key_map,
            current_package_id,
            expr,
        ),
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
            if let Some(modifier) = default_modifier {
                if let Some(payload) = &mut modifier.payload {
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
                if let Some(modifier) = &mut line.modifier {
                    if let Some(payload) = &mut modifier.payload {
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
            }
            Ok(())
        }
        ExecStmt::Bind {
            default_modifier,
            lines,
        } => {
            if let Some(modifier) = default_modifier {
                if let Some(payload) = &mut modifier.payload {
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
                if let Some(modifier) = &mut line.modifier {
                    if let Some(payload) = &mut modifier.payload {
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
            if let Some(modifier) = &mut region.default_modifier {
                if let Some(payload) = &mut modifier.payload {
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
            for line in &mut region.lines {
                rewrite_expr_routine_keys(
                    package_display_names,
                    package_direct_dep_ids,
                    duplicate_keys,
                    routine_key_map,
                    current_package_id,
                    &mut line.value,
                )?;
                if let Some(modifier) = &mut line.modifier {
                    if let Some(payload) = &mut modifier.payload {
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
            }
            Ok(())
        }
        ExecStmt::MemorySpec(spec) => {
            if let Some(modifier) = &mut spec.default_modifier {
                if let Some(payload) = &mut modifier.payload {
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
            for detail in &mut spec.details {
                rewrite_expr_routine_keys(
                    package_display_names,
                    package_direct_dep_ids,
                    duplicate_keys,
                    routine_key_map,
                    current_package_id,
                    &mut detail.value,
                )?;
                if let Some(modifier) = &mut detail.modifier {
                    if let Some(payload) = &mut modifier.payload {
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
        ExecExpr::Int(_) | ExecExpr::Bool(_) | ExecExpr::Str(_) | ExecExpr::Path(_) => Ok(()),
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
            if let Some(modifier) = &mut region.default_modifier {
                if let Some(payload) = &mut modifier.payload {
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
            for line in &mut region.lines {
                rewrite_expr_routine_keys(
                    package_display_names,
                    package_direct_dep_ids,
                    duplicate_keys,
                    routine_key_map,
                    current_package_id,
                    &mut line.value,
                )?;
                if let Some(modifier) = &mut line.modifier {
                    if let Some(payload) = &mut modifier.payload {
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
        kind: HirTypeKind::Apply {
            base: HirPath {
                segments: vec!["Pair".to_string()],
                span: Default::default(),
            },
            args: vec![left, right],
        },
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

fn resolved_module_lang_item_rows(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module: &HirModuleSummary,
) -> Vec<String> {
    module
        .lang_items
        .iter()
        .map(|item| {
            let target = lookup_symbol_path(workspace, resolved_module, &item.target)
                .map(resolved_symbol_path)
                .unwrap_or_else(|| item.target.clone());
            render_lang_item_row(&module.module_id, &item.name, &target)
        })
        .collect()
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

fn render_foreword_arg(arg: &HirForewordArg) -> String {
    match &arg.name {
        Some(name) => format!("{name}=\"{}\"", quote_text(&arg.value)),
        None => format!("\"{}\"", quote_text(&arg.value)),
    }
}

fn render_foreword_row(app: &HirForewordApp) -> String {
    format!(
        "{}({})",
        app.name,
        app.args
            .iter()
            .map(render_foreword_arg)
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn is_identifier_text(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
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
    qualifier: &str,
) -> Option<ResolvedPhraseTarget> {
    if qualifier == "call" {
        let path = flatten_callable_expr_path(subject)?;
        return lookup_symbol_path(scope.workspace, scope.resolved_module, &path).map(|resolved| {
            let routine_key = resolved_symbol_routine_key(&resolved);
            ResolvedPhraseTarget {
                path: resolved_symbol_path(resolved),
                routine_key: Some(routine_key),
                dynamic_dispatch: None,
            }
        });
    }

    if let Some(path) = split_simple_path(qualifier).filter(|path| path.len() > 1)
        && let Some(resolved) = lookup_symbol_path(scope.workspace, scope.resolved_module, &path)
    {
        let routine_key = resolved_symbol_routine_key(&resolved);
        return Some(ResolvedPhraseTarget {
            path: resolved_symbol_path(resolved),
            routine_key: Some(routine_key),
            dynamic_dispatch: None,
        });
    }
    None
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

fn lower_rollup(rollup: &HirCleanupFooter) -> ExecCleanupFooter {
    ExecCleanupFooter {
        kind: rollup.kind.as_str().to_string(),
        binding_id: 0,
        subject: rollup.subject.clone(),
        handler_path: rollup.handler_path.clone(),
        resolved_routine: None,
    }
}

fn lower_phrase_qualifier_kind(qualifier: &str) -> ExecPhraseQualifierKind {
    match qualifier.trim() {
        "call" => ExecPhraseQualifierKind::Call,
        "?" => ExecPhraseQualifierKind::Try,
        ">" => ExecPhraseQualifierKind::Apply,
        ">>" => ExecPhraseQualifierKind::AwaitApply,
        other if other.contains('.') => ExecPhraseQualifierKind::NamedPath,
        _ => ExecPhraseQualifierKind::BareMethod,
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
        HirUnaryOp::BorrowRead => ExecUnaryOp::BorrowRead,
        HirUnaryOp::BorrowMut => ExecUnaryOp::BorrowMut,
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
        qualifier,
        ..
    } = expr
    else {
        return Ok(None);
    };
    if qualifier != "call" {
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
    let HirSymbolBody::Owner { objects, .. } = &resolved.symbol.body else {
        return Ok(None);
    };
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

fn lower_exec_expr(expr: &HirExpr) -> ExecExpr {
    match expr {
        HirExpr::Path { segments } => ExecExpr::Path(segments.clone()),
        HirExpr::BoolLiteral { value } => ExecExpr::Bool(*value),
        HirExpr::IntLiteral { text } => ExecExpr::Int(text.parse().unwrap_or_default()),
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
            qualifier,
            attached,
        } => ExecExpr::Phrase {
            subject: Box::new(lower_exec_expr(subject)),
            args: args.iter().map(lower_phrase_arg_exec).collect(),
            qualifier_kind: lower_phrase_qualifier_kind(qualifier),
            qualifier: qualifier.clone(),
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
            qualifier,
            attached,
        } => {
            let qualifier_kind = lower_phrase_qualifier_kind(qualifier);
            let resolved = match qualifier_kind {
                ExecPhraseQualifierKind::Call | ExecPhraseQualifierKind::NamedPath => {
                    resolve_qualified_phrase_target_path(scope, subject, qualifier)
                }
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
    statements.iter().map(lower_exec_stmt).collect()
}

fn lower_exec_stmt(statement: &HirStatement) -> ExecStmt {
    match &statement.kind {
        HirStatementKind::Let {
            mutable,
            name,
            value,
        } => ExecStmt::Let {
            binding_id: 0,
            mutable: *mutable,
            name: name.clone(),
            value: lower_exec_expr(value),
        },
        HirStatementKind::Expr { expr } => ExecStmt::Expr {
            expr: lower_exec_expr(expr),
            cleanup_footers: statement.cleanup_footers.iter().map(lower_rollup).collect(),
        },
        HirStatementKind::Return { value } => match value.as_ref() {
            Some(value) => ExecStmt::ReturnValue {
                value: lower_exec_expr(value),
            },
            None => ExecStmt::ReturnVoid,
        },
        HirStatementKind::If {
            condition,
            then_branch,
            else_branch,
        } => ExecStmt::If {
            condition: lower_exec_expr(condition),
            then_branch: lower_exec_stmt_block(then_branch),
            else_branch: else_branch
                .as_ref()
                .map(|branch| branch.iter().map(lower_exec_stmt).collect())
                .unwrap_or_default(),
            availability: statement
                .availability
                .iter()
                .map(lower_availability_attachment_exec)
                .collect(),
            cleanup_footers: statement.cleanup_footers.iter().map(lower_rollup).collect(),
        },
        HirStatementKind::While { condition, body } => ExecStmt::While {
            condition: lower_exec_expr(condition),
            body: lower_exec_stmt_block(body),
            availability: statement
                .availability
                .iter()
                .map(lower_availability_attachment_exec)
                .collect(),
            cleanup_footers: statement.cleanup_footers.iter().map(lower_rollup).collect(),
        },
        HirStatementKind::For {
            binding,
            iterable,
            body,
        } => ExecStmt::For {
            binding_id: 0,
            binding: binding.clone(),
            iterable: lower_exec_expr(iterable),
            body: lower_exec_stmt_block(body),
            availability: statement
                .availability
                .iter()
                .map(lower_availability_attachment_exec)
                .collect(),
            cleanup_footers: statement.cleanup_footers.iter().map(lower_rollup).collect(),
        },
        HirStatementKind::Defer { expr } => ExecStmt::Defer(lower_exec_expr(expr)),
        HirStatementKind::Break => ExecStmt::Break,
        HirStatementKind::Continue => ExecStmt::Continue,
        HirStatementKind::Assign { target, op, value } => ExecStmt::Assign {
            target: lower_assign_target_exec(target),
            op: lower_assign_op(*op),
            value: lower_exec_expr(value),
        },
        HirStatementKind::Recycle {
            default_modifier,
            lines,
        } => ExecStmt::Recycle {
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
        },
        HirStatementKind::Bind {
            default_modifier,
            lines,
        } => ExecStmt::Bind {
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
        },
        HirStatementKind::Construct(region) => {
            ExecStmt::Construct(lower_construct_region_exec(region))
        }
        HirStatementKind::MemorySpec(spec) => {
            ExecStmt::MemorySpec(lower_module_memory_spec_exec(spec))
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
    let (lowered_stmt, mut cleanup_bindings) = match &statement.kind {
        HirStatementKind::Let {
            mutable,
            name,
            value,
        } => {
            if let Some(owner_activation) = resolve_owner_activation_expr(scope, value)? {
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
                    ExecStmt::ActivateOwner {
                        owner_path: owner_activation.owner_path,
                        owner_local_name: owner_activation.owner_local_name,
                        binding: Some(name.clone()),
                        object_binding_ids,
                        context: lowered_context,
                    },
                    Vec::new(),
                )
            } else {
                let mut binding_id = 0;
                let lowered_value = lower_exec_expr_resolved(value, scope);
                let lowered = ExecStmt::Let {
                    binding_id,
                    mutable: *mutable,
                    name: name.clone(),
                    value: lowered_value,
                };
                if let Some(ty) = infer_expr_hir_type(scope, value) {
                    binding_id = scope.value_scope.insert(name.clone(), ty);
                }
                let lowered = match lowered {
                    ExecStmt::Let {
                        mutable,
                        name,
                        value,
                        ..
                    } => ExecStmt::Let {
                        binding_id,
                        mutable,
                        name,
                        value,
                    },
                    _ => unreachable!(),
                };
                (lowered, Vec::new())
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
                    ExecStmt::ActivateOwner {
                        owner_path: owner_activation.owner_path,
                        owner_local_name: owner_activation.owner_local_name,
                        binding: None,
                        object_binding_ids,
                        context: lowered_context,
                    },
                    Vec::new(),
                )
            } else {
                let statement_cleanup_cleanup_footers = effective_cleanup_cleanup_footers(
                    &statement.cleanup_footers,
                    inherited_cleanup_cleanup_footers,
                );
                (
                    ExecStmt::Expr {
                        expr: lower_exec_expr_resolved(expr, scope),
                        cleanup_footers: lower_resolved_statement_cleanup_footers(
                            scope,
                            statement_cleanup_cleanup_footers,
                            Vec::new(),
                        )?,
                    },
                    Vec::new(),
                )
            }
        }
        HirStatementKind::Return { value } => (
            match value.as_ref() {
                Some(value) => ExecStmt::ReturnValue {
                    value: lower_exec_expr_resolved(value, scope),
                },
                None => ExecStmt::ReturnVoid,
            },
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
                ExecStmt::If {
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
                },
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
                ExecStmt::While {
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
                },
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
            let mut binding_id = 0;
            let mut rollup_bindings = Vec::new();
            if let Some(ty) = infer_iterable_binding_type(scope, iterable) {
                binding_id = body_scope.value_scope.insert(binding.clone(), ty.clone());
                push_cleanup_binding_candidate(
                    &mut rollup_bindings,
                    binding.clone(),
                    binding_id,
                    ty,
                );
            }
            let body_block = lower_exec_stmt_block_resolved_with_cleanup_candidates(
                body,
                &mut body_scope,
                statement_cleanup_cleanup_footers,
            )?;
            rollup_bindings.extend(body_block.cleanup_bindings.iter().cloned());
            (
                ExecStmt::For {
                    binding_id,
                    binding: binding.clone(),
                    iterable: lower_exec_expr_resolved(iterable, scope),
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
                        rollup_bindings.clone(),
                    )?,
                },
                rollup_bindings,
            )
        }
        HirStatementKind::Defer { expr } => (
            ExecStmt::Defer(lower_exec_expr_resolved(expr, scope)),
            Vec::new(),
        ),
        HirStatementKind::Break => (ExecStmt::Break, Vec::new()),
        HirStatementKind::Continue => (ExecStmt::Continue, Vec::new()),
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
            (lowered, Vec::new())
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
                ExecStmt::Recycle {
                    default_modifier,
                    lines: lowered_lines,
                },
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
                ExecStmt::Bind {
                    default_modifier,
                    lines: lowered_lines,
                },
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
            (lowered, Vec::new())
        }
        HirStatementKind::MemorySpec(spec) => (
            ExecStmt::MemorySpec(ExecMemorySpecDecl {
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
            }),
            Vec::new(),
        ),
    };
    collect_lowered_cleanup_bindings(scope, &lowered_stmt, &mut cleanup_bindings);
    Ok(LoweredExecBlock {
        statements: vec![lowered_stmt],
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
    let HirSymbolBody::Owner { objects, exits } = &symbol.body else {
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
                holds: owner_exit.holds.clone(),
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
    let HirSymbolBody::Owner { objects, exits } = &symbol.body else {
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
                holds: owner_exit.holds.clone(),
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
    IrRoutine {
        package_id: module_id.split('.').next().unwrap_or(module_id).to_string(),
        module_id: module_id.to_string(),
        routine_key,
        symbol_name: symbol.name.clone(),
        symbol_kind: symbol.kind.as_str().to_string(),
        exported: symbol.exported,
        is_async: symbol.is_async,
        type_params: symbol.type_params.clone(),
        behavior_attrs: lower_behavior_attrs(symbol),
        params: lower_routine_params(symbol),
        return_type: symbol.return_type.as_ref().map(lower_symbol_routine_type),
        intrinsic_impl: symbol.intrinsic_impl.clone(),
        impl_target_type: impl_decl.map(|decl| lower_symbol_routine_type(&decl.target_type)),
        impl_trait_path: impl_decl
            .and_then(|decl| decl.trait_path.as_ref().map(canonical_impl_trait_path)),
        availability: symbol
            .availability
            .iter()
            .map(lower_availability_attachment_exec)
            .collect(),
        foreword_rows: symbol.forewords.iter().map(render_foreword_row).collect(),
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
    let mut scope = ResolvedRenderScope::new(
        workspace,
        resolved_module,
        symbol.where_clause.clone(),
        &symbol.type_params,
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
        type_params: symbol.type_params.clone(),
        behavior_attrs: lower_behavior_attrs(symbol),
        params: lower_routine_params_resolved(workspace, resolved_module, symbol, &scope),
        return_type: symbol
            .return_type
            .as_ref()
            .map(|ty| lower_resolved_routine_type(workspace, resolved_module, ty)),
        intrinsic_impl: symbol.intrinsic_impl.clone(),
        impl_target_type: impl_decl
            .map(|decl| lower_resolved_routine_type(workspace, resolved_module, &decl.target_type)),
        impl_trait_path: impl_decl
            .and_then(|decl| decl.trait_path.as_ref().map(canonical_impl_trait_path)),
        availability: symbol
            .availability
            .iter()
            .map(|attachment| lower_availability_attachment_exec_resolved(attachment, &scope))
            .collect::<Result<Vec<_>, _>>()?,
        foreword_rows: symbol.forewords.iter().map(render_foreword_row).collect(),
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
        entrypoints,
        routines,
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
                    module
                        .lang_items
                        .iter()
                        .map(|item| {
                            render_lang_item_row(&module.module_id, &item.name, &item.target)
                        })
                        .collect()
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

#[cfg(test)]
mod tests {
    use super::{
        ExecBindLineKind, ExecExpr, ExecStmt, IrModule, lower_hir, lower_package,
        lower_workspace_package, lower_workspace_package_with_resolution,
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
}
