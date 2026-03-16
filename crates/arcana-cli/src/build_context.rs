use arcana_package::{BuildExecutionContext, BuildTarget};

pub(crate) fn build_execution_context_for_target(
    target: &BuildTarget,
) -> Result<BuildExecutionContext, String> {
    match target {
        BuildTarget::InternalAot | BuildTarget::WindowsExe | BuildTarget::WindowsDll => {
            Ok(BuildExecutionContext::default())
        }
        BuildTarget::Other(other) => Err(format!("unsupported build target `{other}`")),
    }
}
