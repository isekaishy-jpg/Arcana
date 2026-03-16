use arcana_ir::{IrEntrypoint, IrModule, IrPackage, IrPackageModule, IrRoutine};

use crate::artifact::{
    AOT_INTERNAL_FORMAT, AotArtifact, AotEntrypointArtifact, AotPackageArtifact,
    AotPackageModuleArtifact, AotRoutineArtifact,
};

pub fn compile_module(module: &IrModule) -> AotArtifact {
    AotArtifact {
        format: AOT_INTERNAL_FORMAT.to_string(),
        symbol_count: module.symbol_count,
        item_count: module.item_count,
    }
}

fn compile_module_artifact(module: &IrPackageModule) -> AotPackageModuleArtifact {
    let compiled = compile_module(&IrModule {
        symbol_count: module.symbol_count,
        item_count: module.item_count,
    });
    AotPackageModuleArtifact {
        module_id: module.module_id.clone(),
        symbol_count: compiled.symbol_count,
        item_count: compiled.item_count,
        line_count: module.line_count,
        non_empty_line_count: module.non_empty_line_count,
        directive_rows: module.directive_rows.clone(),
        lang_item_rows: module.lang_item_rows.clone(),
        exported_surface_rows: module.exported_surface_rows.clone(),
    }
}

fn compile_entrypoint(entrypoint: &IrEntrypoint) -> AotEntrypointArtifact {
    AotEntrypointArtifact {
        module_id: entrypoint.module_id.clone(),
        symbol_name: entrypoint.symbol_name.clone(),
        symbol_kind: entrypoint.symbol_kind.clone(),
        is_async: entrypoint.is_async,
        exported: entrypoint.exported,
    }
}

fn compile_routine(routine: &IrRoutine) -> AotRoutineArtifact {
    AotRoutineArtifact {
        module_id: routine.module_id.clone(),
        routine_key: routine.routine_key.clone(),
        symbol_name: routine.symbol_name.clone(),
        symbol_kind: routine.symbol_kind.clone(),
        exported: routine.exported,
        is_async: routine.is_async,
        type_param_rows: routine.type_param_rows.clone(),
        behavior_attr_rows: routine.behavior_attr_rows.clone(),
        param_rows: routine.param_rows.clone(),
        signature_row: routine.signature_row.clone(),
        intrinsic_impl: routine.intrinsic_impl.clone(),
        impl_target_type: routine.impl_target_type.clone(),
        impl_trait_path: routine.impl_trait_path.clone(),
        foreword_rows: routine.foreword_rows.clone(),
        rollups: routine.rollups.clone(),
        statements: routine.statements.clone(),
    }
}

pub fn compile_package(package: &IrPackage) -> AotPackageArtifact {
    AotPackageArtifact {
        format: AOT_INTERNAL_FORMAT.to_string(),
        package_name: package.package_name.clone(),
        root_module_id: package.root_module_id.clone(),
        direct_deps: package.direct_deps.clone(),
        module_count: package.module_count(),
        dependency_edge_count: package.dependency_edge_count,
        dependency_rows: package.dependency_rows.clone(),
        exported_surface_rows: package.exported_surface_rows.clone(),
        runtime_requirements: package.runtime_requirements.clone(),
        entrypoints: package.entrypoints.iter().map(compile_entrypoint).collect(),
        routines: package.routines.iter().map(compile_routine).collect(),
        modules: package
            .modules
            .iter()
            .map(compile_module_artifact)
            .collect(),
    }
}
