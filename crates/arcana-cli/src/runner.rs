#[cfg(test)]
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use arcana_frontend::check_workspace_graph;
use arcana_package::{
    BuildDisposition, BuildTarget, GrimoireKind, WorkspaceGraph, default_distribution_dir,
    distribution_bundle_is_ready, execute_build_with_context_and_progress, load_workspace_graph,
    plan_build_for_target_with_context, plan_workspace, prepare_build_from_workspace,
    read_lockfile, stage_distribution_bundle, write_lockfile,
};

use crate::build_context::{build_execution_context_for_target, render_build_progress};
use crate::runtime_exec::{self, ProcessContext};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PreparedRun {
    pub target: BuildTarget,
    pub member: String,
    pub artifact_path: PathBuf,
    pub working_dir: Option<PathBuf>,
}

pub(crate) fn run_workspace(
    workspace_dir: PathBuf,
    target: BuildTarget,
    member: Option<String>,
    args: Vec<String>,
) -> Result<i32, String> {
    let prepared = prepare_run_workspace(&workspace_dir, target, member.as_deref())?;
    match prepared.target {
        BuildTarget::InternalAot => {
            runtime_exec::run_plan_file(&prepared.artifact_path, ProcessContext::current(args))
        }
        BuildTarget::WindowsExe => run_native_executable(
            &prepared.artifact_path,
            prepared.working_dir.as_deref(),
            &args,
        ),
        BuildTarget::WindowsDll => {
            Err("`arcana run` does not support the non-executable `windows-dll` target".to_string())
        }
        BuildTarget::Other(_) => Err(format!("unsupported build target `{}`", prepared.target)),
    }
}

pub(crate) fn prepare_run_workspace(
    workspace_dir: &Path,
    target: BuildTarget,
    member: Option<&str>,
) -> Result<PreparedRun, String> {
    #[cfg(test)]
    let _test_guard = crate::heavy_test_mutex()
        .lock()
        .expect("heavy cli test mutex should not be poisoned");
    if matches!(target, BuildTarget::WindowsDll) {
        return Err(
            "`arcana run` does not support the non-executable `windows-dll` target".to_string(),
        );
    }

    let graph = load_workspace_graph(workspace_dir)?;
    let runnable_member = resolve_run_member(&graph, member)?;
    let runnable_member_id = runnable_member.package_id.clone();
    let runnable_member_name = runnable_member.name.clone();

    let order = plan_workspace(&graph)?;
    let checked = check_workspace_graph(&graph)?;
    let (workspace, resolved_workspace) = checked.into_workspace_parts();
    let prepared = prepare_build_from_workspace(&graph, workspace, resolved_workspace)?;
    let lock_path = graph.root_dir.join("Arcana.lock");
    let existing_lock = read_lockfile(&lock_path)?;
    let execution_context = build_execution_context_for_target(&target, None)?;
    let statuses = plan_build_for_target_with_context(
        &graph,
        &order,
        &prepared,
        existing_lock.as_ref(),
        target.clone(),
        &execution_context,
    )?;
    execute_build_with_context_and_progress(
        &graph,
        &prepared,
        &statuses,
        &execution_context,
        |progress| println!("{}", render_build_progress(progress)),
    )?;
    write_lockfile(&graph, &order, &statuses)?;

    let status = statuses
        .iter()
        .find(|status| status.member() == runnable_member_id && status.target() == &target)
        .ok_or_else(|| {
            format!("missing build status for runnable member `{runnable_member_name}`")
        })?;
    let artifact_path = graph.root_dir.join(status.artifact_rel_path());
    let working_dir = match target {
        BuildTarget::WindowsExe => {
            let bundle_dir =
                default_distribution_dir(&graph, &runnable_member_name, &BuildTarget::WindowsExe);
            let root_file_name = artifact_path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| {
                    format!("invalid built artifact path `{}`", artifact_path.display())
                })?
                .to_string();
            if status.disposition() != BuildDisposition::CacheHit
                || !distribution_bundle_is_ready(&bundle_dir, &root_file_name)
            {
                let bundle = stage_distribution_bundle(
                    &graph,
                    &statuses,
                    &runnable_member_id,
                    &BuildTarget::WindowsExe,
                    &bundle_dir,
                )?;
                Some(bundle.bundle_dir)
            } else {
                Some(bundle_dir)
            }
        }
        _ => None,
    };
    let artifact_path =
        match &working_dir {
            Some(bundle_dir) => bundle_dir.join(artifact_path.file_name().ok_or_else(|| {
                format!("invalid built artifact path `{}`", artifact_path.display())
            })?),
            None => artifact_path,
        };
    Ok(PreparedRun {
        target,
        member: runnable_member_name,
        artifact_path,
        working_dir,
    })
}

fn resolve_run_member<'a>(
    graph: &'a WorkspaceGraph,
    requested_member: Option<&str>,
) -> Result<&'a arcana_package::WorkspaceMember, String> {
    let member = match requested_member {
        Some(name) => graph
            .member(name)
            .ok_or_else(|| format!("workspace has no member `{name}`"))?,
        None => default_run_member(graph)?,
    };
    if member.kind != GrimoireKind::App {
        return Err(format!(
            "member `{}` is `{}` and cannot be run directly",
            member.name,
            member.kind.as_str()
        ));
    }
    Ok(member)
}

fn default_run_member(graph: &WorkspaceGraph) -> Result<&arcana_package::WorkspaceMember, String> {
    if let Some(root_member) = graph.member(&graph.root_name) {
        return Ok(root_member);
    }
    let app_members = graph
        .members
        .iter()
        .filter(|member| member.kind == GrimoireKind::App)
        .collect::<Vec<_>>();
    match app_members.as_slice() {
        [member] => Ok(*member),
        [] => Err("workspace has no runnable app member".to_string()),
        _ => Err("workspace has multiple app members; pass `--member <name>`".to_string()),
    }
}

fn run_native_executable(
    exe_path: &Path,
    working_dir: Option<&Path>,
    args: &[String],
) -> Result<i32, String> {
    let mut command = Command::new(exe_path);
    command.args(args);
    if let Some(working_dir) = working_dir {
        command.current_dir(working_dir);
    }
    let status = command.status().map_err(|e| {
        format!(
            "failed to run emitted executable `{}`: {e}",
            exe_path.display()
        )
    })?;
    Ok(status.code().unwrap_or(1))
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn temp_dir(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let dir = repo_root()
            .join("target")
            .join("arcana-cli-runner-tests")
            .join(format!("{label}_{unique}"));
        fs::create_dir_all(&dir).expect("temp dir should be created");
        dir
    }

    fn write_file(path: &Path, text: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("parent directories should be created");
        }
        fs::write(path, text).expect("file should write");
    }

    fn write_app_workspace(dir: &Path, body: &str) {
        write_file(&dir.join("book.toml"), "name = \"app\"\nkind = \"app\"\n");
        write_file(&dir.join("src/shelf.arc"), body);
        write_file(&dir.join("src/types.arc"), "// types\n");
    }

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("repo root should exist")
            .to_path_buf()
    }

    fn add_std_dep(dir: &Path) {
        let std_path = repo_root()
            .join("std")
            .display()
            .to_string()
            .replace('\\', "/");
        fs::write(
            dir.join("book.toml"),
            format!(
                "name = \"app\"\nkind = \"app\"\n\n[deps]\nstd = {{ path = \"{std_path}\" }}\n"
            ),
        )
        .expect("book manifest should write");
    }

    #[test]
    fn run_workspace_executes_internal_aot_artifact() {
        let dir = temp_dir("run_internal");
        write_app_workspace(&dir, "fn main() -> Int:\n    return 7\n");

        let code = run_workspace(dir.clone(), BuildTarget::internal_aot(), None, Vec::new())
            .expect("internal run should succeed");
        assert_eq!(code, 7);

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn prepare_run_workspace_builds_windows_exe_artifact() {
        let dir = temp_dir("prepare_windows_exe");
        write_app_workspace(&dir, "fn main() -> Int:\n    return 7\n");

        let prepared = prepare_run_workspace(&dir, BuildTarget::windows_exe(), None)
            .expect("windows exe run should build");
        let code = run_native_executable(
            &prepared.artifact_path,
            prepared.working_dir.as_deref(),
            &["alpha".to_string()],
        )
        .expect("native exe should run");
        assert_eq!(code, 7);

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn run_workspace_windows_exe_uses_staged_bundle_directory() {
        let dir = temp_dir("run_windows_bundle");
        write_app_workspace(
            &dir,
            concat!(
                "import std.fs\n",
                "fn main() -> Int:\n",
                "    if std.fs.exists :: \"app.exe\" :: call:\n",
                "        return 0\n",
                "    return 9\n",
            ),
        );
        add_std_dep(&dir);

        let code = run_workspace(dir.clone(), BuildTarget::windows_exe(), None, Vec::new())
            .expect("windows exe run should succeed");
        assert_eq!(code, 0);

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn prepare_run_workspace_reuses_existing_windows_bundle_when_fresh() {
        let dir = temp_dir("prepare_windows_bundle_reuse");
        write_app_workspace(&dir, "fn main() -> Int:\n    return 7\n");

        let first = prepare_run_workspace(&dir, BuildTarget::windows_exe(), None)
            .expect("first windows exe run should build");
        let bundle_dir = first
            .working_dir
            .clone()
            .expect("windows exe run should stage a bundle");
        let marker = bundle_dir.join("runner-reuse-marker.txt");
        fs::write(&marker, "keep\n").expect("marker file should write");

        let second = prepare_run_workspace(&dir, BuildTarget::windows_exe(), None)
            .expect("second windows exe run should reuse staged bundle");

        assert_eq!(second.working_dir.as_deref(), Some(bundle_dir.as_path()));
        assert!(
            marker.is_file(),
            "fresh bundle reuse should not clear existing staged bundle contents"
        );

        let _ = fs::remove_dir_all(&dir);
    }
}
