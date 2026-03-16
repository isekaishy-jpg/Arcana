use crate::emit::{AotEmitTarget, AotPackageEmission};
use crate::native_lowering::build_native_lowering_plan;
use crate::native_plan::NativePackagePlan;
use crate::rust_codegen::{generate_windows_dll_project, generate_windows_exe_project};
use crate::rust_toolchain::{compile_rust_native_project, default_native_project_dir};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NativeBackendKind {
    GeneratedRust,
}

pub(crate) fn emit_native_package(plan: NativePackagePlan) -> Result<AotPackageEmission, String> {
    emit_native_package_with_backend(plan, NativeBackendKind::GeneratedRust)
}

fn emit_native_package_with_backend(
    plan: NativePackagePlan,
    backend: NativeBackendKind,
) -> Result<AotPackageEmission, String> {
    match backend {
        NativeBackendKind::GeneratedRust => emit_generated_rust_package(plan),
    }
}

fn emit_generated_rust_package(plan: NativePackagePlan) -> Result<AotPackageEmission, String> {
    let lowering = build_native_lowering_plan(&plan)?;
    let project_dir = default_native_project_dir(plan.target, &plan.artifact.package_name);
    let project = match plan.target {
        AotEmitTarget::WindowsExeBundle => {
            generate_windows_exe_project(&project_dir, &plan, &lowering)?
        }
        AotEmitTarget::WindowsDllBundle => {
            generate_windows_dll_project(&project_dir, &plan, &lowering)?
        }
        AotEmitTarget::InternalArtifact => {
            return Err("internal artifacts do not have a native backend".to_string());
        }
    };
    compile_rust_native_project(project, plan)
}
