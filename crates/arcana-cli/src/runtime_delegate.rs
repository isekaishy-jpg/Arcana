use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use arcana_package::BuildTarget;

pub(crate) fn maybe_run_launch_bundle_via_runner() -> Result<Option<i32>, String> {
    let exe_path = env::current_exe()
        .map_err(|e| format!("failed to resolve current executable for launcher mode: {e}"))?;
    let launch_path = launch_manifest_path(&exe_path);
    if !launch_path.is_file() {
        return Ok(None);
    }
    let runner = runner_executable_path(&exe_path);
    ensure_runner_exists(&runner)?;
    let mut command = Command::new(&runner);
    command.arg("launch-self");
    command.arg(&exe_path);
    command.arg(&launch_path);
    command.arg("--");
    command.args(env::args().skip(1));
    let status = command.status().map_err(|e| {
        format!(
            "failed to delegate launch bundle execution to `{}`: {e}",
            runner.display()
        )
    })?;
    Ok(Some(status.code().unwrap_or(1)))
}

pub(crate) fn run_workspace_via_runner(
    workspace_dir: PathBuf,
    target: BuildTarget,
    member: Option<String>,
    args: Vec<String>,
) -> Result<i32, String> {
    let exe_path = env::current_exe()
        .map_err(|e| format!("failed to resolve current executable for runtime delegation: {e}"))?;
    let runner = runner_executable_path(&exe_path);
    ensure_runner_exists(&runner)?;
    let mut command = Command::new(&runner);
    command.arg("run-workspace");
    command.arg(&workspace_dir);
    command.arg("--target");
    command.arg(target.to_string());
    if let Some(member) = member {
        command.arg("--member");
        command.arg(member);
    }
    if !args.is_empty() {
        command.arg("--");
        command.args(args);
    }
    let status = command.status().map_err(|e| {
        format!(
            "failed to delegate workspace run to `{}`: {e}",
            runner.display()
        )
    })?;
    Ok(status.code().unwrap_or(1))
}

fn launch_manifest_path(exe_path: &Path) -> PathBuf {
    let file_name = exe_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("artifact");
    exe_path.with_file_name(format!("{file_name}.arcana-launch.toml"))
}

fn runner_executable_path(exe_path: &Path) -> PathBuf {
    let file_name = exe_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("arcana");
    let runner_name = if let Some(stripped) = file_name.strip_suffix(".exe") {
        format!("{stripped}-runner.exe")
    } else {
        "arcana-runner".to_string()
    };
    exe_path.with_file_name(runner_name)
}

fn ensure_runner_exists(runner: &Path) -> Result<(), String> {
    if runner.is_file() {
        return Ok(());
    }
    Err(format!(
        "runtime delegation requires `{}`. build it once with `cargo build -p arcana-cli --features runtime-runner --bin arcana-runner`",
        runner.display()
    ))
}
