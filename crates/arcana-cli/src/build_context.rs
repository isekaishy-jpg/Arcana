use arcana_package::{BuildExecutionContext, BuildTarget};

pub(crate) fn build_execution_context_for_target(
    target: &BuildTarget,
    product: Option<String>,
) -> Result<BuildExecutionContext, String> {
    if product.is_some() && !matches!(target, BuildTarget::WindowsDll) {
        return Err(
            "`--product` is only supported for the `windows-dll` export compatibility target"
                .to_string(),
        );
    }
    match target {
        BuildTarget::InternalAot | BuildTarget::WindowsExe | BuildTarget::WindowsDll => {
            Ok(BuildExecutionContext::with_selected_product(product))
        }
        BuildTarget::Other(other) => Err(format!("unsupported build target `{other}`")),
    }
}
