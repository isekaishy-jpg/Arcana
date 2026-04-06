use std::collections::BTreeMap;

use arcana_aot::{AotEntrypointArtifact, AotNativeCallbackArtifact, AotRoutineArtifact};
use arcana_ir::{
    ExecAvailabilityAttachment as ParsedAvailabilityAttachment,
    ExecCleanupFooter as ParsedCleanupFooter, ExecStmt as ParsedStmt, IrRoutineParam,
    IrRoutineType, RUNTIME_MAIN_ENTRYPOINT_NAME, validate_runtime_main_entry_contract,
};
use serde::{Deserialize, Serialize};

pub type RuntimeParamPlan = IrRoutineParam;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeRoutinePlan {
    pub package_id: String,
    pub module_id: String,
    pub routine_key: String,
    pub symbol_name: String,
    pub symbol_kind: String,
    pub exported: bool,
    pub is_async: bool,
    pub type_params: Vec<String>,
    pub behavior_attrs: BTreeMap<String, String>,
    pub params: Vec<RuntimeParamPlan>,
    pub return_type: Option<IrRoutineType>,
    pub intrinsic_impl: Option<String>,
    pub native_impl: Option<String>,
    pub impl_target_type: Option<IrRoutineType>,
    pub impl_trait_path: Option<Vec<String>>,
    pub availability: Vec<ParsedAvailabilityAttachment>,
    pub cleanup_footers: Vec<ParsedCleanupFooter>,
    pub statements: Vec<ParsedStmt>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeNativeCallbackPlan {
    pub package_id: String,
    pub module_id: String,
    pub name: String,
    pub params: Vec<RuntimeParamPlan>,
    pub return_type: Option<IrRoutineType>,
    pub target: Vec<String>,
    pub target_routine_key: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeEntrypointPlan {
    pub package_id: String,
    pub module_id: String,
    pub symbol_name: String,
    pub symbol_kind: String,
    pub is_async: bool,
    pub exported: bool,
    pub routine_index: usize,
}

pub(crate) fn lower_routine(routine: &AotRoutineArtifact) -> RuntimeRoutinePlan {
    RuntimeRoutinePlan {
        package_id: routine.package_id.clone(),
        module_id: routine.module_id.clone(),
        routine_key: routine.routine_key.clone(),
        symbol_name: routine.symbol_name.clone(),
        symbol_kind: routine.symbol_kind.clone(),
        exported: routine.exported,
        is_async: routine.is_async,
        type_params: routine.type_params.clone(),
        behavior_attrs: routine.behavior_attrs.clone(),
        params: routine.params.clone(),
        return_type: routine.return_type.clone(),
        intrinsic_impl: routine.intrinsic_impl.clone(),
        native_impl: routine.native_impl.clone(),
        impl_target_type: routine.impl_target_type.clone(),
        impl_trait_path: routine.impl_trait_path.clone(),
        availability: routine.availability.clone(),
        cleanup_footers: routine.cleanup_footers.clone(),
        statements: routine.statements.clone(),
    }
}

pub(crate) fn lower_native_callback(
    callback: &AotNativeCallbackArtifact,
) -> RuntimeNativeCallbackPlan {
    RuntimeNativeCallbackPlan {
        package_id: callback.package_id.clone(),
        module_id: callback.module_id.clone(),
        name: callback.name.clone(),
        params: callback.params.clone(),
        return_type: callback.return_type.clone(),
        target: callback.target.clone(),
        target_routine_key: callback.target_routine_key.clone(),
    }
}

pub(crate) fn lower_entrypoint(
    entrypoint: &AotEntrypointArtifact,
    routines: &[RuntimeRoutinePlan],
) -> Result<RuntimeEntrypointPlan, String> {
    let (routine_index, routine) = routines
        .iter()
        .enumerate()
        .find(|(_, routine)| {
            routine.package_id == entrypoint.package_id
                && routine.module_id == entrypoint.module_id
                && routine.symbol_name == entrypoint.symbol_name
                && routine.symbol_kind == entrypoint.symbol_kind
        })
        .ok_or_else(|| {
            format!(
                "entrypoint `{}` in module `{}` has no lowered runtime routine",
                entrypoint.symbol_name, entrypoint.module_id
            )
        })?;
    if entrypoint.symbol_kind == "fn" && entrypoint.symbol_name == RUNTIME_MAIN_ENTRYPOINT_NAME {
        validate_runtime_main_entry_contract(routine.params.len(), routine.return_type.as_ref())?;
    }
    Ok(RuntimeEntrypointPlan {
        package_id: entrypoint.package_id.clone(),
        module_id: entrypoint.module_id.clone(),
        symbol_name: entrypoint.symbol_name.clone(),
        symbol_kind: entrypoint.symbol_kind.clone(),
        is_async: entrypoint.is_async,
        exported: entrypoint.exported,
        routine_index,
    })
}

pub(crate) fn render_runtime_signature_text(routine: &RuntimeRoutinePlan) -> String {
    let mut rendered = String::new();
    if routine.is_async {
        rendered.push_str("async ");
    }
    if routine.symbol_kind == "system" {
        rendered.push_str("system ");
    } else {
        rendered.push_str("fn ");
    }
    rendered.push_str(&routine.symbol_name);
    if !routine.type_params.is_empty() {
        rendered.push('[');
        rendered.push_str(&routine.type_params.join(", "));
        rendered.push(']');
    }
    rendered.push('(');
    rendered.push_str(
        &routine
            .params
            .iter()
            .map(|param| {
                let mut piece = String::new();
                if let Some(mode) = &param.mode {
                    piece.push_str(mode);
                    piece.push(' ');
                }
                piece.push_str(&param.name);
                piece.push_str(": ");
                piece.push_str(&param.ty.render());
                piece
            })
            .collect::<Vec<_>>()
            .join(", "),
    );
    rendered.push(')');
    if let Some(return_type) = &routine.return_type {
        rendered.push_str(" -> ");
        rendered.push_str(&return_type.render());
    }
    rendered.push(':');
    rendered
}
