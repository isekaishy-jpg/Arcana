use std::env;
use std::fs;
use std::path::PathBuf;
#[cfg(test)]
use std::sync::{Mutex, OnceLock};

use arcana_frontend::{check_path, check_workspace_graph, check_workspace_path};
use arcana_package::{
    BuildTarget, execute_build_with_context_and_progress, load_workspace_graph, parse_build_target,
    plan_build_for_target_with_context, plan_workspace, prepare_build_from_workspace,
    publish_workspace_member, read_lockfile, write_lockfile,
};

mod build_context;
mod package_cmd;
mod runtime_delegate;

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
    if let Some(code) = runtime_delegate::maybe_run_launch_bundle_via_runner()? {
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
        "test" => {
            let rest = args.collect::<Vec<_>>();
            run_test_command(&rest)
        }
        "foreword" => {
            let rest = args.collect::<Vec<_>>();
            run_foreword_command(&rest)
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
            runtime_delegate::run_workspace_via_runner(
                PathBuf::from(path),
                target,
                member,
                run_args,
            )
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
    let summary = if fs::metadata(&path)
        .map_err(|err| format!("failed to read `{}`: {err}", path.display()))?
        .is_dir()
    {
        let checked = check_workspace_path(&path)?;
        for warning in checked.warnings() {
            println!("warning: {}", warning.render());
        }
        checked.into_summary()
    } else {
        check_path(&path)?
    };
    println!(
        "ok: {} (packages: {}, modules: {}, directives: {}, symbols: {}, warnings: {})",
        path.display(),
        summary.package_count,
        summary.module_count,
        summary.directive_count,
        summary.symbol_count,
        summary.warning_count,
    );
    Ok(0)
}

fn run_test_command(args: &[String]) -> Result<i32, String> {
    let output = collect_test_command_output(args)?;
    if !output.is_empty() {
        println!("{output}");
    }
    Ok(0)
}

fn collect_test_command_output(args: &[String]) -> Result<String, String> {
    if args.len() != 2 || args[0] != "--list" {
        return Err("usage: arcana test --list <grimoire-dir>".to_string());
    }
    let checked = check_workspace_path(&PathBuf::from(&args[1]))?;
    let mut lines = checked
        .warnings()
        .iter()
        .map(|warning| format!("warning: {}", warning.render()))
        .collect::<Vec<_>>();
    let mut tests = checked.discovered_tests().to_vec();
    tests.sort_by(|left, right| {
        left.package_id
            .cmp(&right.package_id)
            .then_with(|| left.module_id.cmp(&right.module_id))
            .then_with(|| left.symbol_name.cmp(&right.symbol_name))
    });
    let output = render_test_listing(&tests);
    if !output.is_empty() {
        lines.push(output);
    }
    Ok(lines.join("\n"))
}

fn run_foreword_command(args: &[String]) -> Result<i32, String> {
    let output = collect_foreword_command_output(args)?;
    if !output.is_empty() {
        println!("{output}");
    }
    Ok(0)
}

fn collect_foreword_command_output(args: &[String]) -> Result<String, String> {
    let Some(subcommand) = args.first().map(String::as_str) else {
        return Err("usage: arcana foreword <list|show|index> ...".to_string());
    };
    match subcommand {
        "list" => {
            if args.len() < 2 || args.len() > 4 {
                return Err("usage: arcana foreword list <path> [--format json]".to_string());
            }
            let format_json = args.len() == 4 && args[2] == "--format" && args[3] == "json";
            if args.len() == 4 && !format_json {
                return Err("usage: arcana foreword list <path> [--format json]".to_string());
            }
            let checked = check_workspace_path(&PathBuf::from(&args[1]))?;
            let mut entries = checked.foreword_catalog().to_vec();
            entries.sort_by(|left, right| {
                left.exposed_name
                    .cmp(&right.exposed_name)
                    .then_with(|| left.provider_package_id.cmp(&right.provider_package_id))
            });
            render_foreword_list_output(&entries, format_json)
        }
        "show" => {
            if args.len() < 3 || args.len() > 5 {
                return Err(
                    "usage: arcana foreword show <qualified-name> <path> [--format json]"
                        .to_string(),
                );
            }
            let format_json = args.len() == 5 && args[3] == "--format" && args[4] == "json";
            if args.len() == 5 && !format_json {
                return Err(
                    "usage: arcana foreword show <qualified-name> <path> [--format json]"
                        .to_string(),
                );
            }
            let checked = check_workspace_path(&PathBuf::from(&args[2]))?;
            let entry = checked
                .foreword_catalog()
                .iter()
                .find(|entry| entry.exposed_name == args[1] || entry.qualified_name == args[1])
                .ok_or_else(|| format!("foreword `{}` was not found", args[1]))?;
            render_foreword_show_output(entry, format_json)
        }
        "index" => {
            if args.len() < 2 || args.len() > 5 {
                return Err(
                    "usage: arcana foreword index <path> [--public-only] [--format json]"
                        .to_string(),
                );
            }
            let mut public_only = false;
            let mut format_json = false;
            let mut index = 2usize;
            while index < args.len() {
                match args[index].as_str() {
                    "--public-only" => {
                        public_only = true;
                        index += 1;
                    }
                    "--format" => {
                        let Some(value) = args.get(index + 1) else {
                            return Err(
                                "usage: arcana foreword index <path> [--public-only] [--format json]"
                                    .to_string(),
                            );
                        };
                        if value != "json" {
                            return Err(
                                "usage: arcana foreword index <path> [--public-only] [--format json]"
                                    .to_string(),
                            );
                        }
                        format_json = true;
                        index += 2;
                    }
                    _ => {
                        return Err(
                            "usage: arcana foreword index <path> [--public-only] [--format json]"
                                .to_string(),
                        );
                    }
                }
            }
            let checked = check_workspace_path(&PathBuf::from(&args[1]))?;
            let mut entries = checked
                .foreword_index()
                .iter()
                .filter(|entry| !public_only || entry.public)
                .cloned()
                .collect::<Vec<_>>();
            entries.sort_by(|left, right| {
                left.qualified_name
                    .cmp(&right.qualified_name)
                    .then_with(|| left.module_id.cmp(&right.module_id))
                    .then_with(|| left.target_path.cmp(&right.target_path))
                    .then_with(|| left.target_kind.cmp(&right.target_kind))
                    .then_with(|| left.entry_kind.cmp(&right.entry_kind))
            });
            render_foreword_index_output(&entries, format_json)
        }
        _ => Err("usage: arcana foreword <list|show|index> ...".to_string()),
    }
}

fn render_test_listing(tests: &[arcana_frontend::DiscoveredTest]) -> String {
    tests
        .iter()
        .map(|test| {
            format!(
                "{}::{}::{}",
                test.package_id, test.module_id, test.symbol_name
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_foreword_list_output(
    entries: &[arcana_frontend::ForewordCatalogEntry],
    format_json: bool,
) -> Result<String, String> {
    if format_json {
        serde_json::to_string_pretty(entries).map_err(|err| format!("failed to encode JSON: {err}"))
    } else {
        Ok(entries
            .iter()
            .map(|entry| entry.exposed_name.clone())
            .collect::<Vec<_>>()
            .join("\n"))
    }
}

fn render_foreword_show_output(
    entry: &arcana_frontend::ForewordCatalogEntry,
    format_json: bool,
) -> Result<String, String> {
    if format_json {
        serde_json::to_string_pretty(entry).map_err(|err| format!("failed to encode JSON: {err}"))
    } else {
        let mut lines = vec![
            format!("name = {}", entry.exposed_name),
            format!("provider = {}", entry.provider_package_id),
            format!("definition = {}", entry.qualified_name),
            format!("tier = {}", entry.tier),
            format!("visibility = {}", entry.visibility),
            format!("action = {}", entry.action),
            format!("retention = {}", entry.retention),
            format!("targets = {}", entry.targets.join(", ")),
        ];
        if let Some(namespace) = &entry.diagnostic_namespace {
            lines.push(format!("diagnostic_namespace = {namespace}"));
        }
        if let Some(handler) = &entry.handler {
            lines.push(format!("handler = {handler}"));
        }
        Ok(lines.join("\n"))
    }
}

fn render_foreword_index_output(
    entries: &[arcana_frontend::ForewordIndexEntry],
    format_json: bool,
) -> Result<String, String> {
    if format_json {
        serde_json::to_string_pretty(entries).map_err(|err| format!("failed to encode JSON: {err}"))
    } else {
        Ok(entries
            .iter()
            .map(|entry| {
                let mut line = format!(
                    "{} {} {} {} {}",
                    entry.qualified_name,
                    entry.entry_kind,
                    entry.target_kind,
                    entry.target_path,
                    entry.retention
                );
                if let Some(generated_by) = &entry.generated_by {
                    line.push_str(&format!(
                        " generated_by={} from {}",
                        generated_by.resolved_name, generated_by.owner_path
                    ));
                }
                line
            })
            .collect::<Vec<_>>()
            .join("\n"))
    }
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
    println!("  arcana test --list <grimoire-dir>");
    println!("  arcana foreword list <path> [--format json]");
    println!("  arcana foreword show <qualified-name> <path> [--format json]");
    println!("  arcana foreword index <path> [--public-only] [--format json]");
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{
        collect_foreword_command_output, collect_test_command_output, heavy_test_mutex,
        render_foreword_index_output, render_foreword_list_output, render_foreword_show_output,
        render_test_listing,
    };

    #[test]
    fn render_test_listing_sorts_to_expected_lines() {
        let output = render_test_listing(&[
            arcana_frontend::DiscoveredTest {
                package_id: "app".to_string(),
                module_id: "app.main".to_string(),
                symbol_name: "alpha".to_string(),
            },
            arcana_frontend::DiscoveredTest {
                package_id: "app".to_string(),
                module_id: "app.main".to_string(),
                symbol_name: "beta".to_string(),
            },
        ]);
        assert_eq!(output, "app::app.main::alpha\napp::app.main::beta");
    }

    #[test]
    fn render_foreword_list_and_show_support_json() {
        let entry = arcana_frontend::ForewordCatalogEntry {
            provider_package_id: "tool.pkg".to_string(),
            exposed_name: "tool.meta.trace".to_string(),
            qualified_name: "tool.meta.trace".to_string(),
            tier: "basic".to_string(),
            visibility: "public".to_string(),
            action: "metadata".to_string(),
            retention: "runtime".to_string(),
            targets: vec!["fn".to_string(), "param".to_string()],
            diagnostic_namespace: Some("tool.meta".to_string()),
            handler: None,
        };

        let list_text =
            render_foreword_list_output(std::slice::from_ref(&entry), false).expect("text");
        assert_eq!(list_text, "tool.meta.trace");

        let show_text = render_foreword_show_output(&entry, false).expect("text");
        assert!(show_text.contains("name = tool.meta.trace"));
        assert!(show_text.contains("targets = fn, param"));

        let json = render_foreword_show_output(&entry, true).expect("json");
        let parsed = serde_json::from_str::<serde_json::Value>(&json).expect("valid json");
        assert_eq!(parsed["qualified_name"], "tool.meta.trace");
        assert_eq!(parsed["retention"], "runtime");
    }

    #[test]
    fn render_foreword_index_output_includes_generated_provenance() {
        let output = render_foreword_index_output(
            &[arcana_frontend::ForewordIndexEntry {
                entry_kind: "generated".to_string(),
                qualified_name: "tool.exec.rewrite".to_string(),
                package_id: "app.pkg".to_string(),
                module_id: "app.main".to_string(),
                target_kind: "fn".to_string(),
                target_path: "app.main.helper".to_string(),
                retention: "runtime".to_string(),
                args: vec!["slot=\"menu\"".to_string()],
                public: true,
                generated_by: Some(arcana_frontend::ForewordGeneratedBy {
                    applied_name: "tool.exec.rewrite".to_string(),
                    resolved_name: "tool.exec.rewrite".to_string(),
                    provider_package_id: "tool.pkg".to_string(),
                    owner_kind: "fn".to_string(),
                    owner_path: "app.main.main".to_string(),
                    args: vec!["slot=\"menu\"".to_string()],
                }),
            }],
            false,
        )
        .expect("text");
        assert!(output.contains("tool.exec.rewrite generated fn app.main.helper runtime"));
        assert!(output.contains("generated_by=tool.exec.rewrite from app.main.main"));

        let json = render_foreword_index_output(
            &[arcana_frontend::ForewordIndexEntry {
                entry_kind: "generated".to_string(),
                qualified_name: "tool.exec.rewrite".to_string(),
                package_id: "app.pkg".to_string(),
                module_id: "app.main".to_string(),
                target_kind: "fn".to_string(),
                target_path: "app.main.helper".to_string(),
                retention: "runtime".to_string(),
                args: vec![],
                public: true,
                generated_by: Some(arcana_frontend::ForewordGeneratedBy {
                    applied_name: "tool.exec.rewrite".to_string(),
                    resolved_name: "tool.exec.rewrite".to_string(),
                    provider_package_id: "tool.pkg".to_string(),
                    owner_kind: "fn".to_string(),
                    owner_path: "app.main.main".to_string(),
                    args: vec![],
                }),
            }],
            true,
        )
        .expect("json");
        let parsed = serde_json::from_str::<serde_json::Value>(&json).expect("valid json");
        assert_eq!(parsed[0]["entry_kind"], "generated");
        assert_eq!(parsed[0]["generated_by"]["owner_path"], "app.main.main");
    }

    #[test]
    fn test_command_lists_workspace_tests_end_to_end() {
        let _guard = heavy_test_mutex()
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let root = make_temp_package(
            "cliapp",
            &[(
                "src/shelf.arc",
                concat!(
                    "foreword cliapp.meta.trace:\n",
                    "    tier = basic\n",
                    "    visibility = public\n",
                    "    targets = [fn]\n",
                    "    retention = runtime\n",
                    "    payload = [label: Str]\n",
                    "#test\n",
                    "#cliapp.meta.trace[label = \"smoke\"]\n",
                    "fn smoke() -> Int:\n",
                    "    return 0\n",
                    "#cliapp.meta.trace[label = \"main\"]\n",
                    "fn main() -> Int:\n",
                    "    return 0\n",
                ),
            )],
        );

        let output =
            collect_test_command_output(&["--list".to_string(), root.display().to_string()])
                .expect("test command should succeed");
        assert!(
            output.trim().ends_with("::cliapp::smoke"),
            "unexpected test listing: {output}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn foreword_commands_render_workspace_catalog_show_and_index_end_to_end() {
        let _guard = heavy_test_mutex()
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let root = make_temp_package(
            "cliapp",
            &[(
                "src/shelf.arc",
                concat!(
                    "foreword cliapp.meta.trace:\n",
                    "    tier = basic\n",
                    "    visibility = public\n",
                    "    targets = [fn]\n",
                    "    retention = runtime\n",
                    "    payload = [label: Str]\n",
                    "#cliapp.meta.trace[label = \"main\"]\n",
                    "fn main() -> Int:\n",
                    "    return 0\n",
                ),
            )],
        );

        let list_output = collect_foreword_command_output(&[
            "list".to_string(),
            root.display().to_string(),
            "--format".to_string(),
            "json".to_string(),
        ])
        .expect("foreword list should succeed");
        let list = serde_json::from_str::<serde_json::Value>(&list_output).expect("valid json");
        assert!(
            list.as_array().is_some_and(|entries| entries
                .iter()
                .any(|entry| { entry["exposed_name"] == "cliapp.meta.trace" })),
            "catalog should include the package foreword"
        );

        let show_output = collect_foreword_command_output(&[
            "show".to_string(),
            "cliapp.meta.trace".to_string(),
            root.display().to_string(),
            "--format".to_string(),
            "json".to_string(),
        ])
        .expect("foreword show should succeed");
        let show = serde_json::from_str::<serde_json::Value>(&show_output).expect("valid json");
        assert_eq!(show["qualified_name"], "cliapp.meta.trace");
        assert_eq!(show["retention"], "runtime");

        let index_output = collect_foreword_command_output(&[
            "index".to_string(),
            root.display().to_string(),
            "--format".to_string(),
            "json".to_string(),
        ])
        .expect("foreword index should succeed");
        let index = serde_json::from_str::<serde_json::Value>(&index_output).expect("valid json");
        assert!(
            index
                .as_array()
                .is_some_and(|entries| entries.iter().any(|entry| {
                    entry["qualified_name"] == "cliapp.meta.trace"
                        && entry["target_path"] == "cliapp.main"
                        && entry["retention"] == "runtime"
                })),
            "index should include the attached runtime-retained foreword"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    fn make_temp_package(name: &str, files: &[(&str, &str)]) -> PathBuf {
        let root = test_temp_dir("arcana-cli-tests", name);
        if root.exists() {
            fs::remove_dir_all(&root).expect("stale temp dir should be removable");
        }
        fs::create_dir_all(root.join("src")).expect("src dir should be creatable");
        fs::write(
            root.join("book.toml"),
            format!("name = \"{name}\"\nkind = \"app\"\n"),
        )
        .expect("manifest should be writable");
        fs::write(root.join("src").join("types.arc"), "").expect("types file should be writable");
        for (relative_path, contents) in files {
            let path = root.join(relative_path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("parent dirs should be creatable");
            }
            fs::write(path, contents).expect("file should be writable");
        }
        root
    }

    fn test_temp_dir(prefix: &str, name: &str) -> PathBuf {
        repo_root()
            .parent()
            .expect("repo root parent should exist")
            .join("target")
            .join(prefix)
            .join(format!("{}-{name}", unique_test_id()))
    }

    fn unique_test_id() -> u128 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should advance")
            .as_nanos()
    }

    fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .canonicalize()
            .expect("repo root should resolve")
    }
}
