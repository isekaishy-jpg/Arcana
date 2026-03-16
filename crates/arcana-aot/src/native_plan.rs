use arcana_ir::IrPackage;

use crate::artifact::AotPackageArtifact;
use crate::emit::{AotEmitContext, AotEmitTarget};
use crate::native_abi::{NativeExport, collect_native_exports};
use crate::{compile_package, render_package_artifact};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeLaunchPlan {
    Executable { main_routine_key: String },
    DynamicLibrary { exports: Vec<NativeExport> },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativePackagePlan {
    pub target: AotEmitTarget,
    pub root_artifact_file_name: String,
    pub artifact: AotPackageArtifact,
    pub artifact_text: String,
    pub launch: NativeLaunchPlan,
}

pub fn build_native_package_plan(
    target: AotEmitTarget,
    package: &IrPackage,
    context: &AotEmitContext,
) -> Result<NativePackagePlan, String> {
    let root_artifact_file_name = context.root_artifact_file_name.clone().ok_or_else(|| {
        format!(
            "{} emission requires a root artifact file name",
            target.format()
        )
    })?;
    let artifact = compile_package(package);
    let artifact_text = render_package_artifact(&artifact);
    let launch = match target {
        AotEmitTarget::WindowsExeBundle => NativeLaunchPlan::Executable {
            main_routine_key: find_main_routine_key(&artifact)?,
        },
        AotEmitTarget::WindowsDllBundle => NativeLaunchPlan::DynamicLibrary {
            exports: collect_native_exports(&artifact)?,
        },
        AotEmitTarget::InternalArtifact => {
            return Err("internal artifacts do not produce a native package plan".to_string());
        }
    };
    Ok(NativePackagePlan {
        target,
        root_artifact_file_name,
        artifact,
        artifact_text,
        launch,
    })
}

fn find_main_routine_key(artifact: &AotPackageArtifact) -> Result<String, String> {
    let main_entrypoints = artifact
        .entrypoints
        .iter()
        .filter(|entry| entry.symbol_kind == "fn" && entry.symbol_name == "main")
        .collect::<Vec<_>>();
    let [entrypoint] = main_entrypoints.as_slice() else {
        return Err(format!(
            "windows-exe target requires exactly one main entrypoint in package `{}`",
            artifact.package_name
        ));
    };
    let main_routines = artifact
        .routines
        .iter()
        .filter(|routine| {
            routine.module_id == entrypoint.module_id
                && routine.symbol_kind == entrypoint.symbol_kind
                && routine.symbol_name == entrypoint.symbol_name
        })
        .collect::<Vec<_>>();
    let [routine] = main_routines.as_slice() else {
        return Err(format!(
            "windows-exe target could not resolve a unique main routine for package `{}`",
            artifact.package_name
        ));
    };
    Ok(routine.routine_key.clone())
}
