use std::fs;
use std::path::Path;
use std::sync::OnceLock;

use arcana_aot::{AotPackageArtifact, parse_package_artifact, render_package_artifact};
use sha2::{Digest, Sha256};

use crate::{BuildTarget, GrimoireKind, PackageResult};

#[cfg(test)]
pub fn current_build_toolchain() -> PackageResult<String> {
    current_build_toolchain_for_target(&BuildTarget::internal_aot())
}

pub fn current_build_toolchain_for_target(target: &BuildTarget) -> PackageResult<String> {
    match target {
        BuildTarget::InternalAot => {
            static TOOLCHAIN: OnceLock<PackageResult<String>> = OnceLock::new();
            TOOLCHAIN
                .get_or_init(|| compute_current_build_toolchain(target))
                .clone()
        }
        BuildTarget::Other(_) => compute_current_build_toolchain(target),
    }
}

fn compute_current_build_toolchain(target: &BuildTarget) -> PackageResult<String> {
    let exe = std::env::current_exe()
        .map_err(|e| format!("failed to resolve current toolchain binary: {e}"))?;
    let bytes = fs::read(&exe).map_err(|e| {
        format!(
            "failed to read current toolchain binary `{}`: {e}",
            exe.display()
        )
    })?;
    let mut hasher = Sha256::new();
    hasher.update(b"arcana_driver_binary_v1\n");
    hasher.update(b"arcana_build_target_v1\n");
    hasher.update(target.key().as_bytes());
    hasher.update(&bytes);
    Ok(format!("binary-sha256:{:x}", hasher.finalize()))
}

pub fn render_cached_artifact(
    member: &str,
    kind: &GrimoireKind,
    fingerprint: &str,
    api_fingerprint: &str,
    target: &BuildTarget,
    toolchain: &str,
    artifact: &AotPackageArtifact,
    artifact_hash: &str,
) -> String {
    format!(
        concat!(
            "member = \"{}\"\n",
            "kind = \"{}\"\n",
            "fingerprint = \"{}\"\n",
            "api_fingerprint = \"{}\"\n",
            "target = \"{}\"\n",
            "toolchain = \"{}\"\n",
            "artifact_hash = \"{}\"\n",
            "{}"
        ),
        escape_toml(member),
        kind.as_str(),
        escape_toml(fingerprint),
        escape_toml(api_fingerprint),
        target,
        escape_toml(toolchain),
        escape_toml(artifact_hash),
        render_package_artifact(artifact)
    )
}

pub fn cached_artifact_matches_status(
    path: &Path,
    expected_member: &str,
    expected_kind: &GrimoireKind,
    expected_fingerprint: &str,
    expected_api_fingerprint: &str,
    expected_target: &BuildTarget,
    expected_format: &str,
    expected_toolchain: &str,
    expected_artifact_hash: &str,
) -> bool {
    if expected_artifact_hash.is_empty() {
        return false;
    }
    let Ok(text) = fs::read_to_string(path) else {
        return false;
    };
    let Ok(value) = text.parse::<toml::Value>() else {
        return false;
    };
    let Some(table) = value.as_table() else {
        return false;
    };
    let matches_header = table.get("member").and_then(toml::Value::as_str) == Some(expected_member)
        && table.get("kind").and_then(toml::Value::as_str) == Some(expected_kind.as_str())
        && table.get("fingerprint").and_then(toml::Value::as_str) == Some(expected_fingerprint)
        && table.get("api_fingerprint").and_then(toml::Value::as_str)
            == Some(expected_api_fingerprint)
        && table.get("target").and_then(toml::Value::as_str) == Some(expected_target.key())
        && table.get("toolchain").and_then(toml::Value::as_str) == Some(expected_toolchain)
        && table.get("artifact_hash").and_then(toml::Value::as_str) == Some(expected_artifact_hash);
    if !matches_header {
        return false;
    }
    let Ok(artifact) = parse_package_artifact(&text) else {
        return false;
    };
    artifact.format == expected_format
        && artifact.package_name == expected_member
        && artifact_body_hash(&artifact) == expected_artifact_hash
}

pub fn artifact_body_hash_for_path(path: &Path) -> PackageResult<String> {
    let text = fs::read_to_string(path)
        .map_err(|e| format!("failed to read artifact `{}`: {e}", path.display()))?;
    let artifact = parse_package_artifact(&text)
        .map_err(|e| format!("failed to parse artifact `{}`: {e}", path.display()))?;
    Ok(artifact_body_hash(&artifact))
}

pub fn artifact_body_hash(artifact: &AotPackageArtifact) -> String {
    let rendered = render_package_artifact(artifact);
    let mut hasher = Sha256::new();
    hasher.update(b"arcana_aot_body_v1\n");
    hasher.update(rendered.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

fn escape_toml(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}
