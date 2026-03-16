use crate::artifact::{AOT_INTERNAL_FORMAT, AotPackageArtifact};
use crate::validate::validate_package_artifact;

pub fn render_package_artifact(artifact: &AotPackageArtifact) -> String {
    toml::to_string(artifact).expect("backend artifact should serialize")
}

pub fn parse_package_artifact(text: &str) -> Result<AotPackageArtifact, String> {
    let artifact = toml::from_str::<AotPackageArtifact>(text)
        .map_err(|err| format!("failed to parse backend artifact: {err}"))?;
    if artifact.format != AOT_INTERNAL_FORMAT {
        return Err(format!(
            "unsupported backend artifact format `{}`; expected `{AOT_INTERNAL_FORMAT}`",
            artifact.format
        ));
    }
    validate_package_artifact(&artifact)?;
    Ok(artifact)
}
