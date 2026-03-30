use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::emit::AotPackageEmission;
use crate::{AotEmitTarget, NativePackagePlan};

use super::rust_codegen::RustNativeProject;

struct NativeProjectDirGuard {
    project_dir: PathBuf,
}

impl NativeProjectDirGuard {
    fn new(project_dir: PathBuf) -> Self {
        Self { project_dir }
    }
}

impl Drop for NativeProjectDirGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.project_dir);
    }
}

pub fn compile_rust_native_project(
    project: RustNativeProject,
    plan: NativePackagePlan,
) -> Result<AotPackageEmission, String> {
    ensure_native_target_host_supported(plan.target)?;
    let _cleanup = NativeProjectDirGuard::new(project.project_dir.clone());
    write_rust_native_project(&project)?;
    let target_dir = project.project_dir.join("target");
    let status = Command::new("cargo")
        .current_dir(&project.project_dir)
        .arg("build")
        .arg("-q")
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

    let output_path = target_output_path(&target_dir, &project.output_name, plan.target);
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
    if project.project_dir.exists() {
        fs::remove_dir_all(&project.project_dir).map_err(|e| {
            format!(
                "failed to clear native project directory `{}`: {e}",
                project.project_dir.display()
            )
        })?;
    }
    fs::create_dir_all(project.project_dir.join("src")).map_err(|e| {
        format!(
            "failed to create native project directory `{}`: {e}",
            project.project_dir.display()
        )
    })?;
    fs::write(project.project_dir.join("Cargo.toml"), &project.cargo_toml).map_err(|e| {
        format!(
            "failed to write native Cargo.toml `{}`: {e}",
            project.project_dir.join("Cargo.toml").display()
        )
    })?;
    fs::write(
        project.project_dir.join("src").join("artifact.toml"),
        &project.artifact_text,
    )
    .map_err(|e| {
        format!(
            "failed to write native artifact source `{}`: {e}",
            project
                .project_dir
                .join("src")
                .join("artifact.toml")
                .display()
        )
    })?;
    if let Some(main_rs) = &project.main_rs {
        fs::write(project.project_dir.join("src").join("main.rs"), main_rs).map_err(|e| {
            format!(
                "failed to write native main.rs `{}`: {e}",
                project.project_dir.join("src").join("main.rs").display()
            )
        })?;
    }
    if let Some(build_rs) = &project.build_rs {
        fs::write(project.project_dir.join("build.rs"), build_rs).map_err(|e| {
            format!(
                "failed to write native build.rs `{}`: {e}",
                project.project_dir.join("build.rs").display()
            )
        })?;
    }
    if let Some(lib_rs) = &project.lib_rs {
        fs::write(project.project_dir.join("src").join("lib.rs"), lib_rs).map_err(|e| {
            format!(
                "failed to write native lib.rs `{}`: {e}",
                project.project_dir.join("src").join("lib.rs").display()
            )
        })?;
    }
    Ok(())
}

fn target_output_path(target_dir: &Path, output_name: &str, target: AotEmitTarget) -> PathBuf {
    let profile_dir = target_dir.join("debug");
    match target {
        AotEmitTarget::WindowsExeBundle => profile_dir.join(output_name),
        AotEmitTarget::WindowsDllBundle => profile_dir.join(output_name),
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
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    repo_root()
        .join("target")
        .join("arcana-native-projects")
        .join(format!("{}_{}_{}", target.format(), package_name, unique))
}

#[cfg(test)]
mod tests {
    use super::{NativeProjectDirGuard, repo_root};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        repo_root()
            .join("target")
            .join("arcana-aot-rust-toolchain-tests")
            .join(format!("{label}_{unique}"))
    }

    #[test]
    fn native_project_dir_guard_removes_partial_project_tree_on_drop() {
        let dir = temp_dir("cleanup_guard");
        fs::create_dir_all(dir.join("src")).expect("project dir should be created");
        fs::write(dir.join("src").join("artifact.toml"), "format = \"test\"\n")
            .expect("artifact should write");

        {
            let _guard = NativeProjectDirGuard::new(dir.clone());
            assert!(dir.exists(), "guard should observe created project dir");
        }

        assert!(
            !dir.exists(),
            "guard should remove partially written native project dir"
        );
    }
}
