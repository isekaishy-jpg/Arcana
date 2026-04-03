use arcana_ir::{
    IrEntrypoint, IrModule, IrOwnerDecl, IrOwnerExit, IrOwnerObject, IrPackage, IrPackageModule,
    IrRoutine,
};

use crate::artifact::{
    AOT_INTERNAL_FORMAT, AotArtifact, AotEntrypointArtifact, AotOwnerArtifact,
    AotOwnerExitArtifact, AotOwnerObjectArtifact, AotPackageArtifact, AotPackageModuleArtifact,
    AotRoutineArtifact,
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
        package_id: module.package_id.clone(),
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
        package_id: entrypoint.package_id.clone(),
        module_id: entrypoint.module_id.clone(),
        symbol_name: entrypoint.symbol_name.clone(),
        symbol_kind: entrypoint.symbol_kind.clone(),
        is_async: entrypoint.is_async,
        exported: entrypoint.exported,
    }
}

fn compile_routine(routine: &IrRoutine) -> AotRoutineArtifact {
    AotRoutineArtifact {
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
        impl_target_type: routine.impl_target_type.clone(),
        impl_trait_path: routine.impl_trait_path.clone(),
        availability: routine.availability.clone(),
        cleanup_footers: routine.cleanup_footers.clone(),
        statements: routine.statements.clone(),
    }
}

fn compile_owner_object(object: &IrOwnerObject) -> AotOwnerObjectArtifact {
    AotOwnerObjectArtifact {
        type_path: object.type_path.clone(),
        local_name: object.local_name.clone(),
        init_routine_key: object.init_routine_key.clone(),
        init_with_context_routine_key: object.init_with_context_routine_key.clone(),
        resume_routine_key: object.resume_routine_key.clone(),
        resume_with_context_routine_key: object.resume_with_context_routine_key.clone(),
    }
}

fn compile_owner_exit(owner_exit: &IrOwnerExit) -> AotOwnerExitArtifact {
    AotOwnerExitArtifact {
        name: owner_exit.name.clone(),
        condition: owner_exit.condition.clone(),
        holds: owner_exit.holds.clone(),
    }
}

fn compile_owner(owner: &IrOwnerDecl) -> AotOwnerArtifact {
    AotOwnerArtifact {
        package_id: owner.package_id.clone(),
        module_id: owner.module_id.clone(),
        owner_path: owner.owner_path.clone(),
        owner_name: owner.owner_name.clone(),
        context_type: owner.context_type.clone(),
        objects: owner.objects.iter().map(compile_owner_object).collect(),
        exits: owner.exits.iter().map(compile_owner_exit).collect(),
    }
}

pub fn compile_package(package: &IrPackage) -> AotPackageArtifact {
    AotPackageArtifact {
        format: AOT_INTERNAL_FORMAT.to_string(),
        package_id: package.package_id.clone(),
        package_name: package.package_name.clone(),
        root_module_id: package.root_module_id.clone(),
        direct_deps: package.direct_deps.clone(),
        direct_dep_ids: package.direct_dep_ids.clone(),
        package_display_names: package.package_display_names.clone(),
        package_direct_dep_ids: package.package_direct_dep_ids.clone(),
        module_count: package.module_count(),
        dependency_edge_count: package.dependency_edge_count,
        dependency_rows: package.dependency_rows.clone(),
        exported_surface_rows: package.exported_surface_rows.clone(),
        runtime_requirements: package.runtime_requirements.clone(),
        foreword_index: package.foreword_index.clone(),
        foreword_registrations: package.foreword_registrations.clone(),
        entrypoints: package.entrypoints.iter().map(compile_entrypoint).collect(),
        routines: package.routines.iter().map(compile_routine).collect(),
        owners: package.owners.iter().map(compile_owner).collect(),
        modules: package
            .modules
            .iter()
            .map(compile_module_artifact)
            .collect(),
    }
}
