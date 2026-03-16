use std::fs;
use std::path::{Path, PathBuf};

use crate::build::BuildStatus;
use crate::build_identity::read_cached_output_metadata;
use crate::{BuildTarget, PackageResult, WorkspaceGraph};

pub const DISTRIBUTION_BUNDLE_FORMAT: &str = "arcana-distribution-bundle-v1";
const DISTRIBUTION_MANIFEST_FILE: &str = "arcana.bundle.toml";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DistributionBundle {
    pub member: String,
    pub target: BuildTarget,
    pub target_format: String,
    pub root_artifact: String,
    pub support_files: Vec<String>,
    pub artifact_hash: String,
    pub toolchain: String,
    pub bundle_dir: PathBuf,
    pub manifest_path: PathBuf,
}

pub fn default_distribution_dir(
    graph: &WorkspaceGraph,
    member: &str,
    target: &BuildTarget,
) -> PathBuf {
    graph.root_dir.join("dist").join(member).join(target.key())
}

pub fn stage_distribution_bundle(
    graph: &WorkspaceGraph,
    statuses: &[BuildStatus],
    member: &str,
    target: &BuildTarget,
    bundle_dir: &Path,
) -> PackageResult<DistributionBundle> {
    let status = statuses
        .iter()
        .find(|status| status.member() == member && status.target() == target)
        .ok_or_else(|| format!("missing build status for member `{member}` target `{target}`"))?;
    let source_root = graph.root_dir.join(status.artifact_rel_path());
    let metadata = read_cached_output_metadata(&source_root, target)?;
    if metadata.member != member {
        return Err(format!(
            "cached build metadata for `{}` reports member `{}`",
            source_root.display(),
            metadata.member
        ));
    }
    if &metadata.target != target {
        return Err(format!(
            "cached build metadata for `{}` reports target `{}` not `{}`",
            source_root.display(),
            metadata.target,
            target
        ));
    }

    fs::create_dir_all(bundle_dir).map_err(|e| {
        format!(
            "failed to create distribution directory `{}`: {e}",
            bundle_dir.display()
        )
    })?;

    let root_file_name = source_root
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("invalid built artifact path `{}`", source_root.display()))?
        .to_string();
    copy_distribution_file(&source_root, &bundle_dir.join(&root_file_name))?;
    for relative_path in &metadata.support_files {
        let source_path = source_root
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(relative_path);
        copy_distribution_file(&source_path, &bundle_dir.join(relative_path))?;
    }

    let manifest_path = bundle_dir.join(DISTRIBUTION_MANIFEST_FILE);
    fs::write(
        &manifest_path,
        render_distribution_manifest(
            member,
            target,
            &metadata.target_format,
            &root_file_name,
            &metadata.support_files,
            &metadata.artifact_hash,
            &metadata.toolchain,
        ),
    )
    .map_err(|e| {
        format!(
            "failed to write distribution manifest `{}`: {e}",
            manifest_path.display()
        )
    })?;

    Ok(DistributionBundle {
        member: member.to_string(),
        target: target.clone(),
        target_format: metadata.target_format,
        root_artifact: root_file_name,
        support_files: metadata.support_files,
        artifact_hash: metadata.artifact_hash,
        toolchain: metadata.toolchain,
        bundle_dir: bundle_dir.to_path_buf(),
        manifest_path,
    })
}

fn copy_distribution_file(source: &Path, destination: &Path) -> PackageResult<()> {
    let bytes = fs::read(source)
        .map_err(|e| format!("failed to read staged file `{}`: {e}", source.display()))?;
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "failed to create distribution subdirectory `{}`: {e}",
                parent.display()
            )
        })?;
    }
    fs::write(destination, bytes).map_err(|e| {
        format!(
            "failed to write distribution file `{}`: {e}",
            destination.display()
        )
    })
}

fn render_distribution_manifest(
    member: &str,
    target: &BuildTarget,
    target_format: &str,
    root_artifact: &str,
    support_files: &[String],
    artifact_hash: &str,
    toolchain: &str,
) -> String {
    let support_files = support_files
        .iter()
        .map(|file| format!("\"{}\"", escape_toml(file)))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        concat!(
            "format = \"{}\"\n",
            "member = \"{}\"\n",
            "target = \"{}\"\n",
            "target_format = \"{}\"\n",
            "root_artifact = \"{}\"\n",
            "artifact_hash = \"{}\"\n",
            "toolchain = \"{}\"\n",
            "support_files = [{}]\n"
        ),
        DISTRIBUTION_BUNDLE_FORMAT,
        escape_toml(member),
        target,
        escape_toml(target_format),
        escape_toml(root_artifact),
        escape_toml(artifact_hash),
        escape_toml(toolchain),
        support_files,
    )
}

fn escape_toml(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}
