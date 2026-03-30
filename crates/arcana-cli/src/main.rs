use std::env;
use std::path::PathBuf;
#[cfg(test)]
use std::sync::{Mutex, OnceLock};

use arcana_frontend::{check_path, check_workspace_graph};
use arcana_package::{
    BuildTarget, execute_build_with_context_and_progress, load_workspace_graph, parse_build_target,
    plan_build_for_target_with_context, plan_workspace, prepare_build_from_workspace,
    publish_workspace_member, read_lockfile, write_lockfile,
};

mod build_context;
mod launcher;
mod package_cmd;
mod runner;
mod runtime_exec;

type ParsedPackageArgs = (BuildTarget, Option<String>, Option<String>, Option<PathBuf>);

#[cfg(test)]
pub(crate) fn heavy_test_mutex() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

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
                    "usage: arcana build <workspace-dir> [--plan] [--target <target>] [--product <name>]".to_string(),
                );
            };
            let mut plan_only = false;
            let mut target = BuildTarget::internal_aot();
            let mut product = None;
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
                                "usage: arcana build <workspace-dir> [--plan] [--target <target>] [--product <name>]"
                                    .to_string(),
                            );
                        };
                        target = parse_build_target(value)?;
                        index += 2;
                    }
                    "--product" => {
                        let Some(value) = rest.get(index + 1) else {
                            return Err(
                                "usage: arcana build <workspace-dir> [--plan] [--target <target>] [--product <name>]"
                                    .to_string(),
                            );
                        };
                        product = Some(value.clone());
                        index += 2;
                    }
                    _ => {
                        return Err(
                            "usage: arcana build <workspace-dir> [--plan] [--target <target>] [--product <name>]"
                                .to_string(),
                        );
                    }
                }
            }
            run_build(PathBuf::from(path), plan_only, target, product)
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
                    "usage: arcana package <workspace-dir> [--target <target>] [--product <name>] [--member <member>] [--out-dir <dir>]"
                        .to_string(),
                );
            };
            let rest = args.collect::<Vec<_>>();
            let (target, product, member, out_dir) = parse_package_args(&rest)?;
            let bundle = package_cmd::package_workspace_with_product(
                PathBuf::from(path),
                target,
                product,
                member,
                out_dir,
            )?;
            println!(
                "packaged {} {} {}",
                bundle.member,
                bundle.target,
                bundle.bundle_dir.display()
            );
            Ok(0)
        }
        "publish" => {
            let Some(path) = args.next() else {
                return Err("usage: arcana publish <workspace-dir> --member <member>".to_string());
            };
            let rest = args.collect::<Vec<_>>();
            let member = parse_publish_args(&rest)?;
            let published = publish_workspace_member(&PathBuf::from(path), &member)?;
            for package in published {
                println!("published {package}");
            }
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

fn run_build(
    workspace_dir: PathBuf,
    plan_only: bool,
    target: BuildTarget,
    product: Option<String>,
) -> Result<i32, String> {
    let graph = load_workspace_graph(&workspace_dir)?;
    let order = plan_workspace(&graph)?;
    if plan_only {
        for member in order {
            match &product {
                Some(product) => println!("{member} {}@{product}", target),
                None => println!("{member} {target}"),
            }
        }
        return Ok(0);
    }

    let checked = check_workspace_graph(&graph)?;
    let (workspace, resolved_workspace) = checked.into_workspace_parts();
    let prepared = prepare_build_from_workspace(&graph, workspace, resolved_workspace)?;
    let lock_path = graph.root_dir.join("Arcana.lock");
    let existing_lock = read_lockfile(&lock_path)?;
    let execution_context =
        build_context::build_execution_context_for_target(&target, product.clone())?;
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
        |progress| println!("{}", build_context::render_build_progress(progress)),
    )?;
    write_lockfile(&graph, &order, &statuses)?;
    println!(
        "{}",
        build_context::render_build_completion(&statuses, &target, product.as_deref())
    );
    Ok(0)
}

fn print_help() {
    println!("arcana");
    println!("  arcana check <path>");
    println!("  arcana build <workspace-dir> [--plan] [--target <target>] [--product <name>]");
    println!("  arcana run <workspace-dir> [--target <target>] [--member <member>] [-- <args...>]");
    println!(
        "  arcana package <workspace-dir> [--target <target>] [--product <name>] [--member <member>] [--out-dir <dir>]"
    );
    println!("  arcana publish <workspace-dir> --member <member>");
    println!(
        "    targets: internal-aot, windows-exe, windows-dll (native product target; legacy export name)"
    );
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

fn parse_package_args(args: &[String]) -> Result<ParsedPackageArgs, String> {
    let usage = "usage: arcana package <workspace-dir> [--target <target>] [--product <name>] [--member <member>] [--out-dir <dir>]";
    let mut target = BuildTarget::internal_aot();
    let mut product = None;
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
            "--product" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(usage.to_string());
                };
                product = Some(value.clone());
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
    Ok((target, product, member, out_dir))
}

fn parse_publish_args(args: &[String]) -> Result<String, String> {
    let usage = "usage: arcana publish <workspace-dir> --member <member>";
    if args.len() != 2 || args[0] != "--member" {
        return Err(usage.to_string());
    }
    Ok(args[1].clone())
}
