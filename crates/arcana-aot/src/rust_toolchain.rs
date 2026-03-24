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
    let runtime_dynamic_libraries =
        collect_runtime_dynamic_libraries(&target_dir, &project.runtime_dynamic_libraries)?;
    let rust_runtime_libraries =
        collect_rust_runtime_dynamic_libraries(&project.runtime_dynamic_libraries)?;
    let mut support_files = project
        .support_files
        .into_iter()
        .map(|(relative_path, bytes)| crate::emit::AotEmissionFile {
            relative_path,
            bytes,
        })
        .collect::<Vec<_>>();
    support_files.extend(
        runtime_dynamic_libraries
            .into_iter()
            .map(|(relative_path, bytes)| crate::emit::AotEmissionFile {
                relative_path,
                bytes,
            }),
    );
    support_files.extend(
        rust_runtime_libraries
            .into_iter()
            .map(|(relative_path, bytes)| crate::emit::AotEmissionFile {
                relative_path,
                bytes,
            }),
    );
    Ok(AotPackageEmission {
        target: plan.target,
        artifact: plan.artifact,
        primary_artifact_body: plan.artifact_text,
        root_artifact_bytes: Some(root_artifact_bytes),
        support_files,
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

fn collect_runtime_dynamic_libraries(
    target_dir: &Path,
    runtime_dynamic_libraries: &[String],
) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut files = Vec::new();
    for file_name in runtime_dynamic_libraries {
        let (relative_path, path) = find_runtime_dynamic_library(target_dir, file_name)?;
        let bytes = fs::read(&path).map_err(|e| {
            format!(
                "failed to read runtime dynamic library `{}`: {e}",
                path.display()
            )
        })?;
        files.push((relative_path, bytes));
    }
    Ok(files)
}

fn find_runtime_dynamic_library(
    target_dir: &Path,
    file_name: &str,
) -> Result<(String, PathBuf), String> {
    let profile_dir = target_dir.join("debug");
    let exact = profile_dir.join(file_name);
    if exact.is_file() {
        return Ok((file_name.to_string(), exact));
    }
    let deps_dir = profile_dir.join("deps");
    if deps_dir.is_dir() {
        let stem = file_name.strip_suffix(".dll").unwrap_or(file_name);
        for entry in fs::read_dir(&deps_dir).map_err(|e| {
            format!(
                "failed to scan runtime dynamic libraries in `{}`: {e}",
                deps_dir.display()
            )
        })? {
            let entry = entry.map_err(|e| format!("failed to read directory entry: {e}"))?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Some(candidate) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if candidate == file_name
                || (candidate.starts_with(&format!("{stem}-")) && candidate.ends_with(".dll"))
            {
                return Ok((candidate.to_string(), path));
            }
        }
    }
    Err(format!(
        "failed to locate runtime dynamic library `{file_name}` under `{}`",
        profile_dir.display()
    ))
}

fn collect_rust_runtime_dynamic_libraries(
    runtime_dynamic_libraries: &[String],
) -> Result<Vec<(String, Vec<u8>)>, String> {
    if runtime_dynamic_libraries.is_empty() || !cfg!(windows) {
        return Ok(Vec::new());
    }
    let sysroot = current_rust_sysroot()?;
    let bin_dir = sysroot.join("bin");
    if !bin_dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    for entry in fs::read_dir(&bin_dir).map_err(|e| {
        format!(
            "failed to scan Rust toolchain runtime libraries in `{}`: {e}",
            bin_dir.display()
        )
    })? {
        let entry = entry.map_err(|e| format!("failed to read directory entry: {e}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !file_name.starts_with("std-") || !file_name.ends_with(".dll") {
            continue;
        }
        let bytes = fs::read(&path).map_err(|e| {
            format!(
                "failed to read Rust runtime dynamic library `{}`: {e}",
                path.display()
            )
        })?;
        files.push((file_name.to_string(), bytes));
    }
    Ok(files)
}

fn current_rust_sysroot() -> Result<PathBuf, String> {
    let output = Command::new("rustc")
        .arg("--print")
        .arg("sysroot")
        .output()
        .map_err(|e| format!("failed to query Rust sysroot with rustc: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "rustc --print sysroot failed with status {}",
            output.status
        ));
    }
    let text = String::from_utf8(output.stdout)
        .map_err(|e| format!("rustc --print sysroot produced invalid UTF-8: {e}"))?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("rustc --print sysroot returned an empty path".to_string());
    }
    Ok(PathBuf::from(trimmed))
}
