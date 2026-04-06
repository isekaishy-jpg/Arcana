use std::collections::BTreeMap;
use std::env;
use std::path::PathBuf;

use arcana_package::{BuildTarget, parse_build_target};

mod build_context;
mod launcher;
mod runner;
mod runtime_exec;

use runtime_exec::ProcessContext;

fn main() {
    let code = match real_main() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("{err}");
            1
        }
    };
    std::process::exit(code);
}

fn real_main() -> Result<i32, String> {
    if let Some(code) = launcher::maybe_run_launch_bundle()? {
        return Ok(code);
    }
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        return Err("usage: arcana-runner <run-workspace|launch-self> ...".to_string());
    };
    match command.as_str() {
        "run-workspace" => {
            let Some(path) = args.next() else {
                return Err(
                    "usage: arcana-runner run-workspace <workspace-dir> [--target <target>] [--member <member>] [-- <args...>]"
                        .to_string(),
                );
            };
            let rest = args.collect::<Vec<_>>();
            let (target, member, run_args) = parse_run_args(&rest)?;
            runner::run_workspace(PathBuf::from(path), target, member, run_args)
        }
        "launch-self" => {
            let Some(exe_path) = args.next() else {
                return Err(
                    "usage: arcana-runner launch-self <exe-path> <launch-manifest> [-- <args...>]"
                        .to_string(),
                );
            };
            let Some(launch_path) = args.next() else {
                return Err(
                    "usage: arcana-runner launch-self <exe-path> <launch-manifest> [-- <args...>]"
                        .to_string(),
                );
            };
            let rest = args.collect::<Vec<_>>();
            let run_args = if let Some(index) = rest.iter().position(|arg| arg == "--") {
                rest[index + 1..].to_vec()
            } else {
                rest
            };
            launcher::run_launch_bundle(
                &PathBuf::from(exe_path),
                &PathBuf::from(launch_path),
                ProcessContext {
                    args: run_args,
                    env: env::vars().collect::<BTreeMap<_, _>>(),
                    cwd: env::current_dir()
                        .map(|path| path.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                },
            )
        }
        other => Err(format!("unknown runtime runner command `{other}`")),
    }
}

fn parse_run_args(args: &[String]) -> Result<(BuildTarget, Option<String>, Vec<String>), String> {
    let usage = "usage: arcana-runner run-workspace <workspace-dir> [--target <target>] [--member <member>] [-- <args...>]";
    let mut target = BuildTarget::internal_aot();
    let mut member = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--" => {
                return Ok((target, member, args[index + 1..].to_vec()));
            }
            "--target" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(usage.to_string());
                };
                target = parse_build_target(value)?;
                index += 2;
            }
            "--member" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(usage.to_string());
                };
                member = Some(value.clone());
                index += 2;
            }
            _ => return Err(usage.to_string()),
        }
    }
    Ok((target, member, Vec::new()))
}
