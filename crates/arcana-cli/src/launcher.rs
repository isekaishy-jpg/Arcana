use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use arcana_runtime::BufferedHost;

use crate::runtime_exec::{self, ProcessContext};

const LAUNCH_BUNDLE_FORMAT: &str = "arcana-launch-v1";

pub fn maybe_run_launch_bundle() -> Result<Option<i32>, String> {
    let exe_path = env::current_exe()
        .map_err(|e| format!("failed to resolve current executable for launcher mode: {e}"))?;
    let launch_path = launch_manifest_path(&exe_path);
    if !launch_path.is_file() {
        return Ok(None);
    }
    run_launch_bundle(
        &exe_path,
        &launch_path,
        ProcessContext::current(env::args().skip(1).collect()),
    )
    .map(Some)
}

pub(crate) fn run_launch_bundle(
    exe_path: &Path,
    launch_path: &Path,
    context: ProcessContext,
) -> Result<i32, String> {
    let (code, host) = execute_launch_bundle(exe_path, launch_path, context)?;
    runtime_exec::flush_buffered_host(&host)?;
    Ok(code)
}

pub(crate) fn execute_launch_bundle(
    exe_path: &Path,
    launch_path: &Path,
    context: ProcessContext,
) -> Result<(i32, BufferedHost), String> {
    let artifact_rel_path = parse_launch_manifest(launch_path)?;
    let base_dir = exe_path.parent().unwrap_or_else(|| Path::new("."));
    let artifact_path = base_dir.join(&artifact_rel_path);
    let plan = runtime_exec::load_plan_file(&artifact_path)?;
    runtime_exec::execute_plan(&plan, context)
}

pub(crate) fn launch_manifest_path(exe_path: &Path) -> PathBuf {
    let file_name = exe_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("artifact");
    exe_path.with_file_name(format!("{file_name}.arcana-launch.toml"))
}

fn parse_launch_manifest(path: &Path) -> Result<String, String> {
    let text = fs::read_to_string(path)
        .map_err(|e| format!("failed to read launcher manifest `{}`: {e}", path.display()))?;
    let value = text.parse::<toml::Value>().map_err(|e| {
        format!(
            "failed to parse launcher manifest `{}`: {e}",
            path.display()
        )
    })?;
    let table = value
        .as_table()
        .ok_or_else(|| format!("launcher manifest `{}` must be a table", path.display()))?;
    let format = table
        .get("format")
        .and_then(toml::Value::as_str)
        .ok_or_else(|| format!("launcher manifest `{}` is missing `format`", path.display()))?;
    if format != LAUNCH_BUNDLE_FORMAT {
        return Err(format!(
            "unsupported launcher manifest format `{format}` in `{}`",
            path.display()
        ));
    }
    table
        .get("artifact")
        .and_then(toml::Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| {
            format!(
                "launcher manifest `{}` is missing `artifact`",
                path.display()
            )
        })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use arcana_aot::{
        AOT_INTERNAL_FORMAT, AotEntrypointArtifact, AotPackageArtifact, AotPackageModuleArtifact,
        AotRoutineArtifact, render_package_artifact,
    };
    use arcana_ir::{ExecExpr, ExecStmt};

    use super::*;

    fn temp_dir(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("arcana_cli_launcher_{label}_{unique}"));
        fs::create_dir_all(&dir).expect("temp dir should be created");
        dir
    }

    fn sample_launch_artifact() -> AotPackageArtifact {
        AotPackageArtifact {
            format: AOT_INTERNAL_FORMAT.to_string(),
            package_name: "app".to_string(),
            root_module_id: "app".to_string(),
            direct_deps: Vec::new(),
            module_count: 1,
            dependency_edge_count: 0,
            dependency_rows: Vec::new(),
            exported_surface_rows: vec!["module=app:export:fn:fn main() -> Int:".to_string()],
            runtime_requirements: Vec::new(),
            entrypoints: vec![AotEntrypointArtifact {
                module_id: "app".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                is_async: false,
                exported: true,
            }],
            routines: vec![AotRoutineArtifact {
                module_id: "app".to_string(),
                routine_key: "app#fn-0".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_param_rows: Vec::new(),
                behavior_attr_rows: Vec::new(),
                param_rows: Vec::new(),
                signature_row: "fn main() -> Int:".to_string(),
                intrinsic_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                foreword_rows: Vec::new(),
                rollups: Vec::new(),
                statements: vec![ExecStmt::ReturnValue {
                    value: ExecExpr::Int(7),
                }],
            }],
            owners: Vec::new(),
            modules: vec![AotPackageModuleArtifact {
                module_id: "app".to_string(),
                symbol_count: 1,
                item_count: 1,
                line_count: 1,
                non_empty_line_count: 1,
                directive_rows: Vec::new(),
                lang_item_rows: Vec::new(),
                exported_surface_rows: vec!["export:fn:fn main() -> Int:".to_string()],
            }],
        }
    }

    #[test]
    fn execute_launch_bundle_runs_embedded_artifact() {
        let dir = temp_dir("run_bundle");
        let exe_path = dir.join("app.exe");
        let launch_path = dir.join("app.exe.arcana-launch.toml");
        let artifact_path = dir.join("app.exe.artifact.toml");
        fs::write(
            &launch_path,
            "format = \"arcana-launch-v1\"\nartifact = \"app.exe.artifact.toml\"\n",
        )
        .expect("launch manifest should write");
        fs::write(
            &artifact_path,
            render_package_artifact(&sample_launch_artifact()),
        )
        .expect("embedded artifact should write");

        let (code, host) = execute_launch_bundle(
            &exe_path,
            &launch_path,
            ProcessContext {
                args: vec!["alpha".to_string()],
                env: BTreeMap::new(),
                cwd: dir.to_string_lossy().into_owned(),
            },
        )
        .expect("launcher should execute");
        assert_eq!(code, 7);
        assert_eq!(host.args, vec!["alpha".to_string()]);

        let _ = fs::remove_dir_all(&dir);
    }
}
