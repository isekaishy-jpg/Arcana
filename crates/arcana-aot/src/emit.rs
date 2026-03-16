use arcana_ir::IrPackage;

use crate::artifact::{AOT_INTERNAL_FORMAT, AotPackageArtifact};
use crate::codec::render_package_artifact;
use crate::compile::compile_package;
use crate::windows_bundle::emit_windows_exe_bundle;
use crate::windows_dll::emit_windows_dll_bundle;

pub const AOT_WINDOWS_EXE_FORMAT: &str = "arcana-native-exe-v1";
pub const AOT_WINDOWS_DLL_FORMAT: &str = "arcana-native-dll-v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AotEmitTarget {
    InternalArtifact,
    WindowsExeBundle,
    WindowsDllBundle,
}

impl AotEmitTarget {
    pub fn format(self) -> &'static str {
        match self {
            Self::InternalArtifact => AOT_INTERNAL_FORMAT,
            Self::WindowsExeBundle => AOT_WINDOWS_EXE_FORMAT,
            Self::WindowsDllBundle => AOT_WINDOWS_DLL_FORMAT,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AotEmissionFile {
    pub relative_path: String,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AotEmitContext {
    pub root_artifact_file_name: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AotPackageEmission {
    pub target: AotEmitTarget,
    pub artifact: AotPackageArtifact,
    pub primary_artifact_body: String,
    pub root_artifact_bytes: Option<Vec<u8>>,
    pub support_files: Vec<AotEmissionFile>,
}

pub fn emit_package(
    target: AotEmitTarget,
    package: &IrPackage,
) -> Result<AotPackageEmission, String> {
    emit_package_with_context(target, package, &AotEmitContext::default())
}

pub fn emit_package_with_context(
    target: AotEmitTarget,
    package: &IrPackage,
    context: &AotEmitContext,
) -> Result<AotPackageEmission, String> {
    match target {
        AotEmitTarget::InternalArtifact => {
            let artifact = compile_package(package);
            let primary_artifact_body = render_package_artifact(&artifact);
            Ok(AotPackageEmission {
                target,
                artifact,
                primary_artifact_body,
                root_artifact_bytes: None,
                support_files: Vec::new(),
            })
        }
        AotEmitTarget::WindowsExeBundle => emit_windows_exe_bundle(package, context),
        AotEmitTarget::WindowsDllBundle => emit_windows_dll_bundle(package, context),
    }
}
