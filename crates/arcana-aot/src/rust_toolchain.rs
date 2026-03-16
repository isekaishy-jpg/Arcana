use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::emit::AotPackageEmission;
use crate::{AotEmitTarget, NativePackagePlan};

use super::rust_codegen::RustNativeProject;

pub fn compile_rust_native_project(
    project: RustNativeProject,
    plan: NativePackagePlan,
) -> Result<AotPackageEmission, String> {
    ensure_native_target_host_supported(plan.target)?;
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
    Ok(AotPackageEmission {
        target: plan.target,
        artifact: plan.artifact,
        primary_artifact_body: plan.artifact_text,
        root_artifact_bytes: Some(root_artifact_bytes),
        support_files: project
            .support_files
            .into_iter()
            .map(|(relative_path, bytes)| crate::emit::AotEmissionFile {
                relative_path,
                bytes,
            })
            .collect(),
    })
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

pub fn default_native_project_dir(target: AotEmitTarget, package_name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "arcana_native_project_{}_{}_{}",
        target.format(),
        package_name,
        unique
    ))
}
