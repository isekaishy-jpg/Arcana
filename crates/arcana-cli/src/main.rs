use std::env;
use std::path::PathBuf;

use arcana_frontend::{check_path, check_workspace_graph};
use arcana_package::{
    BuildDisposition, BuildTarget, execute_build_with_context, load_workspace_graph,
    parse_build_target, plan_build_for_target_with_context, plan_workspace,
    prepare_build_from_workspace, read_lockfile, render_build_summary, write_lockfile,
};

mod build_context;
mod launcher;
mod package_cmd;
mod runner;
mod runtime_exec;

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
        print_help();
        return Ok(0);
    };

    match command.as_str() {
        "help" | "-h" | "--help" => {
            print_help();
            Ok(0)
        }
        "check" => {
            let Some(path) = args.next() else {
                return Err("usage: arcana check <path>".to_string());
            };
            if args.next().is_some() {
                return Err("usage: arcana check <path>".to_string());
            }
            run_check(PathBuf::from(path))
        }
        "build" => {
            let Some(path) = args.next() else {
                return Err(
                    "usage: arcana build <workspace-dir> [--plan] [--target <target>]".to_string(),
                );
            };
            let mut plan_only = false;
            let mut target = BuildTarget::internal_aot();
            let rest = args.collect::<Vec<_>>();
            let mut index = 0;
            while index < rest.len() {
                match rest[index].as_str() {
                    "--plan" => {
                        plan_only = true;
                        index += 1;
                    }
                    "--target" => {
                        let Some(value) = rest.get(index + 1) else {
                            return Err(
                                "usage: arcana build <workspace-dir> [--plan] [--target <target>]"
                                    .to_string(),
                            );
                        };
                        target = parse_build_target(value)?;
                        index += 2;
                    }
                    _ => {
                        return Err(
                            "usage: arcana build <workspace-dir> [--plan] [--target <target>]"
                                .to_string(),
                        );
                    }
                }
            }
            run_build(PathBuf::from(path), plan_only, target)
        }
        "run" => {
            let Some(path) = args.next() else {
                return Err(
                    "usage: arcana run <workspace-dir> [--target <target>] [--member <member>] [-- <args...>]"
                        .to_string(),
                );
            };
            let rest = args.collect::<Vec<_>>();
            let (target, member, run_args) = parse_run_args(&rest)?;
            runner::run_workspace(PathBuf::from(path), target, member, run_args)
        }
        "package" => {
            let Some(path) = args.next() else {
                return Err(
                    "usage: arcana package <workspace-dir> [--target <target>] [--member <member>] [--out-dir <dir>]"
                        .to_string(),
                );
            };
            let rest = args.collect::<Vec<_>>();
            let (target, member, out_dir) = parse_package_args(&rest)?;
            let bundle =
                package_cmd::package_workspace(PathBuf::from(path), target, member, out_dir)?;
            println!(
                "packaged {} {} {}",
                bundle.member,
                bundle.target,
                bundle.bundle_dir.display()
            );
            Ok(0)
        }
        other => Err(format!("unknown command `{other}`")),
    }
}

fn run_check(path: PathBuf) -> Result<i32, String> {
    let summary = check_path(&path)?;
    println!(
        "ok: {} (packages: {}, modules: {}, directives: {}, symbols: {})",
        path.display(),
        summary.package_count,
        summary.module_count,
        summary.directive_count,
        summary.symbol_count
    );
    Ok(0)
}

fn run_build(workspace_dir: PathBuf, plan_only: bool, target: BuildTarget) -> Result<i32, String> {
    let graph = load_workspace_graph(&workspace_dir)?;
    let order = plan_workspace(&graph)?;
    if plan_only {
        for member in order {
            println!("{member} {target}");
        }
        return Ok(0);
    }

    let checked = check_workspace_graph(&graph)?;
    let (workspace, resolved_workspace) = checked.into_workspace_parts();
    let prepared = prepare_build_from_workspace(&graph, workspace, resolved_workspace)?;
    let lock_path = graph.root_dir.join("Arcana.lock");
    let existing_lock = read_lockfile(&lock_path)?;
    let execution_context = build_context::build_execution_context_for_target(&target)?;
    let statuses = plan_build_for_target_with_context(
        &graph,
        &order,
        &prepared,
        existing_lock.as_ref(),
        target,
        &execution_context,
    )?;
    execute_build_with_context(&graph, &prepared, &statuses, &execution_context)?;
    write_lockfile(&graph, &order, &statuses)?;

    for status in &statuses {
        println!(
            "{} {} {} {}",
            match status.disposition() {
                BuildDisposition::Built => "built",
                BuildDisposition::CacheHit => "cache_hit",
            },
            status.member(),
            status.target(),
            status.fingerprint()
        );
    }
    println!("{}", render_build_summary(&statuses, &graph));
    Ok(0)
}

fn print_help() {
    println!("arcana");
    println!("  arcana check <path>");
    println!("  arcana build <workspace-dir> [--plan] [--target <target>]");
    println!("  arcana run <workspace-dir> [--target <target>] [--member <member>] [-- <args...>]");
    println!(
        "  arcana package <workspace-dir> [--target <target>] [--member <member>] [--out-dir <dir>]"
    );
    println!("    targets: internal-aot, windows-exe, windows-dll");
}

fn parse_run_args(args: &[String]) -> Result<(BuildTarget, Option<String>, Vec<String>), String> {
    let usage =
        "usage: arcana run <workspace-dir> [--target <target>] [--member <member>] [-- <args...>]";
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

fn parse_package_args(
    args: &[String],
) -> Result<(BuildTarget, Option<String>, Option<PathBuf>), String> {
    let usage = "usage: arcana package <workspace-dir> [--target <target>] [--member <member>] [--out-dir <dir>]";
    let mut target = BuildTarget::internal_aot();
    let mut member = None;
    let mut out_dir = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
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
            "--out-dir" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(usage.to_string());
                };
                out_dir = Some(PathBuf::from(value));
                index += 2;
            }
            _ => return Err(usage.to_string()),
        }
    }
    Ok((target, member, out_dir))
}
