use crate::AotEmitTarget;
use crate::emit::{AotEmitContext, AotPackageEmission};
use crate::native_backend::emit_native_package;
use crate::native_plan::build_native_package_plan;
use arcana_ir::IrPackage;

pub(crate) fn emit_windows_exe_bundle(
    package: &IrPackage,
    context: &AotEmitContext,
) -> Result<AotPackageEmission, String> {
    let plan = build_native_package_plan(AotEmitTarget::WindowsExeBundle, package, context)?;
    emit_native_package(plan)
}
