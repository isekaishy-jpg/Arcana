use serde::{Deserialize, Serialize};

use crate::RuntimePackagePlan;

pub const RUNTIME_PACKAGE_IMAGE_FORMAT: &str = "arcana-runtime-package-image-v1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct RuntimePackageImage {
    format: String,
    plan: RuntimePackagePlan,
}

pub fn render_runtime_package_image(plan: &RuntimePackagePlan) -> Result<String, String> {
    serde_json::to_string(&RuntimePackageImage {
        format: RUNTIME_PACKAGE_IMAGE_FORMAT.to_string(),
        plan: plan.clone(),
    })
    .map_err(|e| format!("failed to render runtime package image: {e}"))
}

pub fn parse_runtime_package_image(text: &str) -> Result<RuntimePackagePlan, String> {
    let image = serde_json::from_str::<RuntimePackageImage>(text)
        .map_err(|e| format!("failed to parse runtime package image: {e}"))?;
    if image.format != RUNTIME_PACKAGE_IMAGE_FORMAT {
        return Err(format!(
            "unsupported runtime package image format `{}`; expected `{RUNTIME_PACKAGE_IMAGE_FORMAT}`",
            image.format
        ));
    }
    Ok(image.plan)
}
