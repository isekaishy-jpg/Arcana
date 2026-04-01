use std::collections::BTreeMap;
use std::env;
use std::io::{self, Write};
use std::path::Path;

use arcana_runtime::{
    ARCANA_NATIVE_BUNDLE_DIR_ENV, ARCANA_NATIVE_BUNDLE_MANIFEST_ENV, BufferedHost,
    RuntimePackagePlan, execute_main, load_package_plan,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProcessContext {
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub cwd: String,
}

impl ProcessContext {
    pub fn current(args: Vec<String>) -> Self {
        Self {
            args,
            env: env::vars().collect(),
            cwd: env::current_dir()
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_default(),
        }
    }
}

pub(crate) fn run_plan(plan: &RuntimePackagePlan, context: ProcessContext) -> Result<i32, String> {
    let (code, host) = execute_plan(plan, context)?;
    flush_buffered_host(&host)?;
    Ok(code)
}

pub(crate) fn run_plan_file(path: &Path, context: ProcessContext) -> Result<i32, String> {
    let plan = load_plan_file(path)?;
    let mut context = context;
    if let Some(parent) = path.parent() {
        context
            .env
            .entry(ARCANA_NATIVE_BUNDLE_DIR_ENV.to_string())
            .or_insert_with(|| parent.to_string_lossy().into_owned());
    }
    let manifest_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| format!("{name}.arcana-bundle.toml"))
        .unwrap_or_else(|| "arcana.bundle.toml".to_string());
    context
        .env
        .entry(ARCANA_NATIVE_BUNDLE_MANIFEST_ENV.to_string())
        .or_insert_with(|| {
            path.with_file_name(manifest_name)
                .to_string_lossy()
                .into_owned()
        });
    run_plan(&plan, context)
}

pub(crate) fn load_plan_file(path: &Path) -> Result<RuntimePackagePlan, String> {
    load_package_plan(path)
        .map_err(|e| format!("failed to load launch artifact `{}`: {e}", path.display()))
}

pub(crate) fn execute_plan(
    plan: &RuntimePackagePlan,
    context: ProcessContext,
) -> Result<(i32, BufferedHost), String> {
    let mut host = BufferedHost::default();
    host.args = context.args;
    host.env = context.env;
    host.allow_process = true;
    host.cwd = context.cwd;
    let code = execute_main(plan, &mut host)?;
    Ok((code, host))
}

pub(crate) fn flush_buffered_host(host: &BufferedHost) -> Result<(), String> {
    if !host.stdout.is_empty() {
        let mut stdout = io::stdout().lock();
        for chunk in &host.stdout {
            stdout
                .write_all(chunk.as_bytes())
                .map_err(|e| format!("failed to write launcher stdout: {e}"))?;
        }
        stdout
            .flush()
            .map_err(|e| format!("failed to flush launcher stdout: {e}"))?;
    }
    if !host.stderr.is_empty() {
        let mut stderr = io::stderr().lock();
        for chunk in &host.stderr {
            stderr
                .write_all(chunk.as_bytes())
                .map_err(|e| format!("failed to write launcher stderr: {e}"))?;
        }
        stderr
            .flush()
            .map_err(|e| format!("failed to flush launcher stderr: {e}"))?;
    }
    Ok(())
}
