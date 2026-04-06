use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::emit::AotPackageEmission;
use crate::{AotEmitTarget, NativePackagePlan};
use fs2::FileExt;
use sha2::{Digest, Sha256};

use super::rust_codegen::RustNativeProject;

pub fn compile_rust_native_project(
    project: RustNativeProject,
    plan: NativePackagePlan,
) -> Result<AotPackageEmission, String> {
    ensure_native_target_host_supported(plan.target)?;
    let target_dir = default_shared_native_target_dir(plan.target);
    let output_path = target_output_path(&target_dir, &project.cargo_output_name, plan.target);
    let fingerprint = rust_native_project_inputs_fingerprint(&project)?;
    write_rust_native_project(&project)?;
    if output_path.is_file()
        && read_inputs_stamp(&rust_native_project_inputs_stamp_path(&project.project_dir))
            .is_some_and(|existing| existing == fingerprint)
    {
        let root_artifact_bytes = fs::read(&output_path).map_err(|e| {
            format!(
                "failed to read compiled native artifact `{}`: {e}",
                output_path.display()
            )
        })?;
        let support_files = project
            .support_files
            .into_iter()
            .map(|(relative_path, bytes)| crate::emit::AotEmissionFile {
                relative_path,
                bytes,
            })
            .collect::<Vec<_>>();
        return Ok(AotPackageEmission {
            target: plan.target,
            artifact: plan.artifact,
            primary_artifact_body: plan.artifact_text,
            root_artifact_bytes: Some(root_artifact_bytes),
            support_files,
        });
    }
    let _build_lock = acquire_cargo_target_lock(&target_dir)?;
    let status = Command::new("cargo")
        .current_dir(&project.project_dir)
        .arg("build")
        .arg("--message-format")
        .arg("short")
        .arg("--target-dir")
        .arg(&target_dir)
        .status()
        .map_err(|e| {
            format!(
                "failed to invoke cargo for native project `{}`: {e}",
                project.project_dir.display()
            )
        })?;
    if !status.success() {
        return Err(format!(
            "native project build failed for `{}` with status {status}",
            project.project_dir.display()
        ));
    }

    write_inputs_stamp(
        &rust_native_project_inputs_stamp_path(&project.project_dir),
        &fingerprint,
    )?;
    let root_artifact_bytes = fs::read(&output_path).map_err(|e| {
        format!(
            "failed to read compiled native artifact `{}`: {e}",
            output_path.display()
        )
    })?;
    let support_files = project
        .support_files
        .into_iter()
        .map(|(relative_path, bytes)| crate::emit::AotEmissionFile {
            relative_path,
            bytes,
        })
        .collect::<Vec<_>>();
    let emission = AotPackageEmission {
        target: plan.target,
        artifact: plan.artifact,
        primary_artifact_body: plan.artifact_text,
        root_artifact_bytes: Some(root_artifact_bytes),
        support_files,
    };
    Ok(emission)
}

fn ensure_native_target_host_supported(target: AotEmitTarget) -> Result<(), String> {
    if cfg!(windows) {
        return Ok(());
    }
    match target {
        AotEmitTarget::InternalArtifact => Ok(()),
        AotEmitTarget::WindowsExeBundle | AotEmitTarget::WindowsDllBundle => Err(format!(
            "{} currently requires a Windows host toolchain",
            target.format()
        )),
    }
}

fn write_rust_native_project(project: &RustNativeProject) -> Result<(), String> {
    fs::create_dir_all(project.project_dir.join("src")).map_err(|e| {
        format!(
            "failed to create native project directory `{}`: {e}",
            project.project_dir.display()
        )
    })?;
    write_file_if_changed(project.project_dir.join("Cargo.toml"), &project.cargo_toml)?;
    write_file_if_changed(
        project.project_dir.join("src").join("artifact.toml"),
        &project.artifact_text,
    )?;
    if let Some(main_rs) = &project.main_rs {
        write_file_if_changed(project.project_dir.join("src").join("main.rs"), main_rs)?;
    }
    if let Some(build_rs) = &project.build_rs {
        write_file_if_changed(project.project_dir.join("build.rs"), build_rs)?;
    }
    if let Some(lib_rs) = &project.lib_rs {
        write_file_if_changed(project.project_dir.join("src").join("lib.rs"), lib_rs)?;
    }
    Ok(())
}

fn write_file_if_changed(path: PathBuf, content: &str) -> Result<(), String> {
    if fs::read_to_string(&path)
        .ok()
        .is_some_and(|existing| existing == content)
    {
        return Ok(());
    }
    fs::write(&path, content).map_err(|e| format!("failed to write `{}`: {e}", path.display()))
}

fn target_output_path(
    target_dir: &Path,
    cargo_output_name: &str,
    target: AotEmitTarget,
) -> PathBuf {
    let profile_dir = target_dir.join("debug");
    match target {
        AotEmitTarget::WindowsExeBundle => profile_dir.join(format!("{cargo_output_name}.exe")),
        AotEmitTarget::WindowsDllBundle => profile_dir.join(format!("{cargo_output_name}.dll")),
        AotEmitTarget::InternalArtifact => unreachable!(),
    }
}

fn repo_root() -> PathBuf {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .parent()
        .and_then(Path::parent)
        .expect("workspace root should exist")
        .to_path_buf()
}

pub fn default_native_project_dir(target: AotEmitTarget, package_name: &str) -> PathBuf {
    repo_root()
        .join("target")
        .join("arcana-native-projects")
        .join(format!(
            "{}_{}",
            target.format(),
            sanitize_path_component(package_name)
        ))
}

pub fn default_shared_native_target_dir(target: AotEmitTarget) -> PathBuf {
    repo_root()
        .join("target")
        .join("arcana-cargo-targets")
        .join(format!(
            "native-{}",
            sanitize_path_component(target.format())
        ))
}

fn sanitize_path_component(text: &str) -> String {
    text.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}

fn rust_native_project_inputs_stamp_path(project_dir: &Path) -> PathBuf {
    project_dir.join(".arcana-native-project.inputs")
}

fn read_inputs_stamp(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok()
}

fn write_inputs_stamp(path: &Path, fingerprint: &str) -> Result<(), String> {
    fs::write(path, fingerprint).map_err(|e| {
        format!(
            "failed to write native project inputs stamp `{}`: {e}",
            path.display()
        )
    })
}

fn acquire_cargo_target_lock(target_dir: &Path) -> Result<std::fs::File, String> {
    fs::create_dir_all(target_dir).map_err(|e| {
        format!(
            "failed to create shared cargo target directory `{}`: {e}",
            target_dir.display()
        )
    })?;
    let lock_path = target_dir.join(".arcana-cargo-build.lock");
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(|e| {
            format!(
                "failed to open shared cargo lock `{}`: {e}",
                lock_path.display()
            )
        })?;
    file.lock_exclusive().map_err(|e| {
        format!(
            "failed to lock shared cargo target directory `{}`: {e}",
            target_dir.display()
        )
    })?;
    Ok(file)
}

fn rust_native_project_inputs_fingerprint(project: &RustNativeProject) -> Result<String, String> {
    let root = repo_root();
    let mut hasher = Sha256::new();
    hasher.update(b"arcana_rust_native_project_inputs_v1\n");
    hasher.update(project.output_name.as_bytes());
    hasher.update(b"\n--cargo-output-name--\n");
    hasher.update(project.cargo_output_name.as_bytes());
    hasher.update(b"\n--cargo--\n");
    hasher.update(project.cargo_toml.as_bytes());
    hasher.update(b"\n--artifact--\n");
    hasher.update(project.artifact_text.as_bytes());
    if let Some(build_rs) = &project.build_rs {
        hasher.update(b"\n--build-rs--\n");
        hasher.update(build_rs.as_bytes());
    }
    if let Some(main_rs) = &project.main_rs {
        hasher.update(b"\n--main-rs--\n");
        hasher.update(main_rs.as_bytes());
    }
    if let Some(lib_rs) = &project.lib_rs {
        hasher.update(b"\n--lib-rs--\n");
        hasher.update(lib_rs.as_bytes());
    }
    for (path, bytes) in &project.support_files {
        hasher.update(format!("\n--support:{path}--\n").as_bytes());
        hasher.update(bytes);
    }
    fingerprint_path_contents(&root.join("Cargo.toml"), &mut hasher)?;
    fingerprint_path_contents(&root.join("Cargo.lock"), &mut hasher)?;
    fingerprint_tree_contents(&root.join("crates").join("arcana-runtime"), &mut hasher)?;
    fingerprint_tree_contents(&root.join("crates").join("arcana-cabi"), &mut hasher)?;
    fingerprint_tree_contents(&root.join("crates").join("arcana-aot"), &mut hasher)?;
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn fingerprint_tree_contents(path: &Path, hasher: &mut Sha256) -> Result<(), String> {
    if !path.exists() {
        hasher.update(format!("missing:{}\n", path.display()).as_bytes());
        return Ok(());
    }
    let mut entries = fs::read_dir(path)
        .map_err(|e| {
            format!(
                "failed to read `{}` for native project fingerprinting: {e}",
                path.display()
            )
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| {
            format!(
                "failed to enumerate `{}` for native project fingerprinting: {e}",
                path.display()
            )
        })?;
    entries.sort_by_key(|entry| entry.path());
    for entry in entries {
        let entry_path = entry.path();
        let metadata = entry.metadata().map_err(|e| {
            format!(
                "failed to read metadata for `{}`: {e}",
                entry_path.display()
            )
        })?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name == "target" || name == ".git" {
            continue;
        }
        if metadata.is_dir() {
            hasher.update(format!("dir:{}\n", entry_path.display()).as_bytes());
            fingerprint_tree_contents(&entry_path, hasher)?;
        } else if metadata.is_file() {
            fingerprint_path_contents(&entry_path, hasher)?;
        }
    }
    Ok(())
}

fn fingerprint_path_contents(path: &Path, hasher: &mut Sha256) -> Result<(), String> {
    let bytes = fs::read(path)
        .map_err(|e| format!("failed to read `{}` for hashing: {e}", path.display()))?;
    hasher.update(format!("file:{}:{}\n", path.display(), bytes.len()).as_bytes());
    hasher.update(&bytes);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{default_native_project_dir, default_shared_native_target_dir, repo_root};

    #[test]
    fn default_native_project_dir_is_stable_for_same_package() {
        let first = default_native_project_dir(super::AotEmitTarget::WindowsExeBundle, "Demo");
        let second = default_native_project_dir(super::AotEmitTarget::WindowsExeBundle, "Demo");
        assert_eq!(first, second);
        assert!(
            first.starts_with(repo_root().join("target").join("arcana-native-projects")),
            "native project dir should stay under target/arcana-native-projects"
        );
    }

    #[test]
    fn shared_native_target_dir_is_stable_for_same_target() {
        let first = default_shared_native_target_dir(super::AotEmitTarget::WindowsExeBundle);
        let second = default_shared_native_target_dir(super::AotEmitTarget::WindowsExeBundle);
        assert_eq!(first, second);
        assert!(
            first.starts_with(repo_root().join("target").join("arcana-cargo-targets")),
            "shared cargo target dir should stay under target/arcana-cargo-targets"
        );
    }
}
