use std::env;
use std::path::PathBuf;

use arcana_frontend::{check_path, check_workspace_graph};
use arcana_package::{
    BuildDisposition, execute_build, load_workspace_graph, plan_build, plan_workspace,
    prepare_build_from_workspace, read_lockfile, render_build_summary, write_lockfile,
};

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
                return Err("usage: arcana build <workspace-dir> [--plan]".to_string());
            };
            let rest = args.collect::<Vec<_>>();
            let plan_only = match rest.as_slice() {
                [] => false,
                [flag] if flag == "--plan" => true,
                _ => return Err("usage: arcana build <workspace-dir> [--plan]".to_string()),
            };
            run_build(PathBuf::from(path), plan_only)
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

fn run_build(workspace_dir: PathBuf, plan_only: bool) -> Result<i32, String> {
    let graph = load_workspace_graph(&workspace_dir)?;
    let order = plan_workspace(&graph)?;
    if plan_only {
        for member in order {
            println!("{member}");
        }
        return Ok(0);
    }

    let checked = check_workspace_graph(&graph)?;
    let (workspace, resolved_workspace) = checked.into_workspace_parts();
    let prepared = prepare_build_from_workspace(&graph, workspace, resolved_workspace)?;
    let lock_path = graph.root_dir.join("Arcana.lock");
    let existing_lock = read_lockfile(&lock_path)?;
    let statuses = plan_build(&graph, &order, &prepared, existing_lock.as_ref())?;
    execute_build(&graph, &prepared, &statuses)?;
    write_lockfile(&graph, &order, &statuses)?;

    for status in &statuses {
        println!(
            "{} {} {}",
            match status.disposition() {
                BuildDisposition::Built => "built",
                BuildDisposition::CacheHit => "cache_hit",
            },
            status.member(),
            status.fingerprint()
        );
    }
    println!("{}", render_build_summary(&statuses, &graph));
    Ok(0)
}

fn print_help() {
    println!("arcana");
    println!("  arcana check <path>");
    println!("  arcana build <workspace-dir> [--plan]");
}
