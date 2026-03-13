
use super::{
    BufferedEvent, BufferedFrameInput, BufferedHost, ParsedExpr, ParsedPageRollup, ParsedPhraseArg,
    ParsedPhraseQualifierKind, ParsedStmt, RuntimeEntrypointPlan, RuntimeHost, RuntimeOpaqueValue,
    RuntimePackagePlan, RuntimeParamPlan, RuntimeRoutinePlan, RuntimeValue, execute_main,
    execute_routine, load_package_plan, plan_from_artifact, resolve_routine_index,
};
use arcana_aot::{
    AOT_INTERNAL_FORMAT, AotEntrypointArtifact, AotPackageArtifact, AotPackageModuleArtifact,
    AotRoutineArtifact, render_package_artifact,
};
use arcana_frontend::{check_workspace_graph, compute_member_fingerprints_for_checked_workspace};
use arcana_package::{execute_build, load_workspace_graph, plan_build, plan_workspace};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_artifact_path(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should advance")
        .as_nanos();
    std::env::temp_dir().join(format!("arcana_runtime_{label}_{nanos}.toml"))
}

fn temp_workspace_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should advance")
        .as_nanos();
    repo_root()
        .join("target")
        .join(format!("arcana_runtime_{label}_{nanos}"))
}

fn repo_root() -> PathBuf {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .parent()
        .and_then(Path::parent)
        .expect("workspace root should exist")
        .to_path_buf()
}

fn write_file(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("parent directories should be created");
    }
    fs::write(path, text).expect("file should write");
}

fn synthetic_window_canvas_host(fixture_root: &Path) -> BufferedHost {
    let cwd = fixture_root.to_string_lossy().replace('\\', "/");
    BufferedHost {
        cwd: cwd.clone(),
        sandbox_root: cwd,
        monotonic_now_ms: 100,
        monotonic_step_ms: 5,
        next_frame_events: vec![
            BufferedEvent {
                kind: 3,
                a: 1,
                b: 0,
            },
            BufferedEvent {
                kind: 4,
                a: 65,
                b: 0,
            },
        ],
        next_frame_input: BufferedFrameInput {
            key_down: vec![65],
            key_pressed: vec![65],
            mouse_pos: (40, 50),
            mouse_in_window: true,
            ..BufferedFrameInput::default()
        },
        ..BufferedHost::default()
    }
}

fn synthetic_audio_host(fixture_root: &Path) -> BufferedHost {
    let cwd = fixture_root.to_string_lossy().replace('\\', "/");
    BufferedHost {
        cwd: cwd.clone(),
        sandbox_root: cwd,
        ..BufferedHost::default()
    }
}

fn write_host_core_workspace(destination: &Path) {
    write_file(
        &destination.join("book.toml"),
        "name = \"runtime_host_core\"\nkind = \"app\"\n",
    );
    write_file(
        &destination.join("src").join("shelf.arc"),
        concat!(
            "import std.collections.list\n",
            "import std.fs\n",
            "import std.io\n",
            "import std.path\n",
            "import std.text\n",
            "use std.result.Result\n",
            "\n",
            "fn list_arc_files(root: Str) -> List[Str]:\n",
            "    let mut pending = std.collections.list.new[Str] :: :: call\n",
            "    let mut files = std.collections.list.new[Str] :: :: call\n",
            "    pending :: root :: push\n",
            "    while (pending :: :: len) > 0:\n",
            "        let path = pending :: :: pop\n",
            "        if std.fs.is_dir :: path :: call:\n",
            "            let mut entries = match (std.fs.list_dir :: path :: call):\n",
            "                Result.Ok(found) => found\n",
            "                Result.Err(_) => std.collections.list.new[Str] :: :: call\n",
            "            while (entries :: :: len) > 0:\n",
            "                pending :: (entries :: :: pop) :: push\n",
            "            continue\n",
            "        if (std.path.ext :: path :: call) != \"arc\":\n",
            "            continue\n",
            "        files :: path :: push\n",
            "    return files\n",
            "\n",
            "fn read_text_or_empty(path: Str) -> Str:\n",
            "    return match (std.fs.read_text :: path :: call):\n",
            "        Result.Ok(text) => text\n",
            "        Result.Err(_) => \"\"\n",
            "\n",
            "fn main() -> Int:\n",
            "    let root = std.path.cwd :: :: call\n",
            "    let mut files = list_arc_files :: root :: call\n",
            "    let mut count = 0\n",
            "    let mut checksum = 0\n",
            "    while (files :: :: len) > 0:\n",
            "        let file = files :: :: pop\n",
            "        let text = read_text_or_empty :: file :: call\n",
            "        let size = std.text.len_bytes :: text :: call\n",
            "        std.io.print[Str] :: file :: call\n",
            "        count += 1\n",
            "        checksum = ((checksum * 131) + size + 7) % 2147483647\n",
            "    let report_dir = std.path.join :: root, \".arcana\" :: call\n",
            "    let logs_dir = std.path.join :: report_dir, \"logs\" :: call\n",
            "    let report_path = std.path.join :: logs_dir, \"host_core_report.txt\" :: call\n",
            "    std.fs.mkdir_all :: logs_dir :: call\n",
            "    std.fs.write_text :: report_path, \"Arcana Runtime Host Core v1\\n\" :: call\n",
            "    std.io.print[Int] :: count :: call\n",
            "    std.io.print[Int] :: checksum :: call\n",
            "    return 0\n",
        ),
    );
    write_file(
        &destination.join("src").join("types.arc"),
        "// test types\n",
    );
}

fn sample_return_artifact() -> AotPackageArtifact {
    AotPackageArtifact {
        format: AOT_INTERNAL_FORMAT,
        package_name: "hello".to_string(),
        root_module_id: "hello".to_string(),
        direct_deps: vec!["std".to_string()],
        module_count: 1,
        dependency_edge_count: 1,
        dependency_rows: vec!["source=hello:import:std.io:".to_string()],
        exported_surface_rows: vec!["module=hello:export:fn:fn main() -> Int:".to_string()],
        runtime_requirements: vec!["std.io".to_string()],
        entrypoints: vec![AotEntrypointArtifact {
            module_id: "hello".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            is_async: false,
            exported: true,
        }],
        routines: vec![AotRoutineArtifact {
            module_id: "hello".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            exported: true,
            is_async: false,
            type_param_rows: Vec::new(),
            behavior_attr_rows: Vec::new(),
            param_rows: Vec::new(),
            signature_row: "fn main() -> Int:".to_string(),
            intrinsic_impl: None,
            foreword_rows: Vec::new(),
            rollup_rows: Vec::new(),
            statement_rows: vec!["stmt(core=return(int(7)),forewords=[],rollups=[])".to_string()],
        }],
        modules: vec![AotPackageModuleArtifact {
            module_id: "hello".to_string(),
            symbol_count: 1,
            item_count: 2,
            line_count: 2,
            non_empty_line_count: 2,
            directive_rows: vec!["module=hello:import:std.io:".to_string()],
            lang_item_rows: Vec::new(),
            exported_surface_rows: vec!["module=hello:export:fn:fn main() -> Int:".to_string()],
        }],
    }
}

fn sample_print_artifact() -> AotPackageArtifact {
    AotPackageArtifact {
            format: AOT_INTERNAL_FORMAT,
            package_name: "hello".to_string(),
            root_module_id: "hello".to_string(),
            direct_deps: vec!["std".to_string()],
            module_count: 1,
            dependency_edge_count: 2,
            dependency_rows: vec![
                "source=hello:import:std.io:".to_string(),
                "source=hello:use:std.io:io".to_string(),
            ],
            exported_surface_rows: vec![],
            runtime_requirements: vec!["std.io".to_string()],
            entrypoints: vec![AotEntrypointArtifact {
                module_id: "hello".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                is_async: false,
                exported: false,
            }],
            routines: vec![AotRoutineArtifact {
                module_id: "hello".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                exported: false,
                is_async: false,
                type_param_rows: Vec::new(),
                behavior_attr_rows: Vec::new(),
                param_rows: Vec::new(),
                signature_row: "fn main():".to_string(),
                intrinsic_impl: None,
                foreword_rows: Vec::new(),
                rollup_rows: Vec::new(),
                statement_rows: vec![
                    "stmt(core=expr(phrase(subject=generic(expr=member(path(io), print),types=[Str]),args=[str(\"\\\"hello, arcana\\\"\")],qualifier=call,attached=[])),forewords=[],rollups=[])".to_string(),
                ],
            }],
            modules: vec![AotPackageModuleArtifact {
                module_id: "hello".to_string(),
                symbol_count: 1,
                item_count: 4,
                line_count: 4,
                non_empty_line_count: 4,
                directive_rows: vec![
                    "module=hello:import:std.io:".to_string(),
                    "module=hello:use:std.io:io".to_string(),
                ],
                lang_item_rows: Vec::new(),
                exported_surface_rows: Vec::new(),
            }],
        }
}

fn sample_stmt_metadata_artifact() -> AotPackageArtifact {
    AotPackageArtifact {
            format: AOT_INTERNAL_FORMAT,
            package_name: "metadata".to_string(),
            root_module_id: "metadata".to_string(),
            direct_deps: Vec::new(),
            module_count: 1,
            dependency_edge_count: 0,
            dependency_rows: Vec::new(),
            exported_surface_rows: vec!["module=metadata:export:fn:fn main() -> Int:".to_string()],
            runtime_requirements: Vec::new(),
            entrypoints: vec![AotEntrypointArtifact {
                module_id: "metadata".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                is_async: false,
                exported: true,
            }],
            routines: vec![AotRoutineArtifact {
                module_id: "metadata".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_param_rows: Vec::new(),
                behavior_attr_rows: Vec::new(),
                param_rows: Vec::new(),
                signature_row: "fn main() -> Int:".to_string(),
                intrinsic_impl: None,
                foreword_rows: vec!["test()".to_string()],
                rollup_rows: vec!["cleanup:scope:metadata.cleanup".to_string()],
                statement_rows: vec![
                    "stmt(core=return(int(0)),forewords=[only(os=\"windows\")],rollups=[cleanup:scope:metadata.cleanup])".to_string(),
                ],
            }],
            modules: vec![AotPackageModuleArtifact {
                module_id: "metadata".to_string(),
                symbol_count: 1,
                item_count: 1,
                line_count: 1,
                non_empty_line_count: 1,
                directive_rows: Vec::new(),
                lang_item_rows: Vec::new(),
                exported_surface_rows: vec![
                    "module=metadata:export:fn:fn main() -> Int:".to_string(),
                ],
            }],
        }
}

fn sample_attachment_foreword_artifact() -> AotPackageArtifact {
    AotPackageArtifact {
            format: AOT_INTERNAL_FORMAT,
            package_name: "attachment".to_string(),
            root_module_id: "attachment".to_string(),
            direct_deps: vec!["std".to_string()],
            module_count: 1,
            dependency_edge_count: 0,
            dependency_rows: Vec::new(),
            exported_surface_rows: vec![
                "module=attachment:export:fn:fn main() -> Int:".to_string(),
            ],
            runtime_requirements: vec!["std.io".to_string()],
            entrypoints: vec![AotEntrypointArtifact {
                module_id: "attachment".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                is_async: false,
                exported: true,
            }],
            routines: vec![AotRoutineArtifact {
                module_id: "attachment".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_param_rows: Vec::new(),
                behavior_attr_rows: Vec::new(),
                param_rows: Vec::new(),
                signature_row: "fn main() -> Int:".to_string(),
                intrinsic_impl: None,
                foreword_rows: Vec::new(),
                rollup_rows: Vec::new(),
                statement_rows: vec![
                    "stmt(core=let(mutable=true,name=xs,value=collection([int(1)])),forewords=[],rollups=[])".to_string(),
                    "stmt(core=expr(phrase(subject=path(std.kernel.collections.list_push),args=[path(xs)],kind=call,qualifier=call,attached=[chain(int(2),forewords=[inline()])])),forewords=[],rollups=[])".to_string(),
                    "stmt(core=expr(phrase(subject=generic(expr=path(std.io.print),types=[Int]),args=[phrase(subject=path(std.kernel.collections.list_len),args=[path(xs)],kind=call,qualifier=call,attached=[])],kind=call,qualifier=call,attached=[])),forewords=[],rollups=[])".to_string(),
                    "stmt(core=return(int(0)),forewords=[],rollups=[])".to_string(),
                ],
            }],
            modules: vec![AotPackageModuleArtifact {
                module_id: "attachment".to_string(),
                symbol_count: 1,
                item_count: 4,
                line_count: 4,
                non_empty_line_count: 4,
                directive_rows: Vec::new(),
                lang_item_rows: Vec::new(),
                exported_surface_rows: vec![
                    "module=attachment:export:fn:fn main() -> Int:".to_string(),
                ],
            }],
        }
}

#[test]
fn plan_from_artifact_links_entrypoints_to_routines() {
    let plan = plan_from_artifact(&sample_return_artifact()).expect("runtime plan should build");
    assert_eq!(plan.entrypoints.len(), 1);
    assert_eq!(plan.routines.len(), 1);
    assert_eq!(plan.entrypoints[0].routine_index, 0);
    assert_eq!(
        plan.main_entrypoint()
            .map(|entry| entry.symbol_name.as_str()),
        Some("main")
    );
}

#[test]
fn load_package_plan_reads_rendered_backend_artifact() {
    let path = temp_artifact_path("load");
    let rendered = format!(
        "member = \"hello\"\nkind = \"app\"\nfingerprint = \"fp\"\napi_fingerprint = \"api\"\n{}",
        render_package_artifact(&sample_return_artifact())
    );
    fs::write(&path, rendered).expect("artifact should write");
    let plan = load_package_plan(&path).expect("runtime plan should load");
    assert_eq!(plan.package_name, "hello");
    assert_eq!(plan.runtime_requirements, vec!["std.io".to_string()]);
    let _ = fs::remove_file(path);
}

#[test]
fn execute_main_returns_exit_code() {
    let plan = plan_from_artifact(&sample_return_artifact()).expect("runtime plan should build");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");
    assert_eq!(code, 7);
    assert!(host.stdout.is_empty());
}

#[test]
fn execute_main_prints_hello() {
    let plan = plan_from_artifact(&sample_print_artifact()).expect("runtime plan should build");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");
    assert_eq!(code, 0);
    assert_eq!(host.stdout, vec!["hello, arcana".to_string()]);
}

#[test]
fn execute_main_rejects_missing_runtime_requirement() {
    let plan = plan_from_artifact(&sample_print_artifact()).expect("runtime plan should build");
    let mut host = BufferedHost {
        supported_runtime_requirements: Some(["std.args".to_string()].into_iter().collect()),
        ..BufferedHost::default()
    };
    let err = execute_main(&plan, &mut host).expect_err("runtime should reject missing io");
    assert!(
        err.contains("std.io"),
        "expected std.io capability error, got {err}"
    );
}

#[test]
fn plan_from_artifact_accepts_stmt_forewords_and_rollups() {
    let plan =
        plan_from_artifact(&sample_stmt_metadata_artifact()).expect("runtime plan should build");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");
    assert_eq!(code, 0);
}

#[test]
fn execute_main_accepts_attachment_foreword_metadata() {
    let plan = plan_from_artifact(&sample_attachment_foreword_artifact())
        .expect("runtime plan should build");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");
    assert_eq!(code, 0);
    assert_eq!(host.stdout, vec!["2".to_string()]);
}

#[test]
fn execute_main_runs_page_rollups_on_loop_exit_and_try_propagation() {
    let dir = temp_workspace_dir("page_rollups");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_page_rollups\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.io\n",
            "import std.result\n",
            "use std.result.Result\n",
            "fn cleanup(value: Int) -> Int:\n",
            "    std.io.print[Int] :: value :: call\n",
            "    return 0\n",
            "fn maybe(flag: Bool) -> Result[Int, Str]:\n",
            "    if flag:\n",
            "        return Result.Err[Int, Str] :: \"bad\" :: call\n",
            "    return Result.Ok[Int, Str] :: 9 :: call\n",
            "fn run(seed: Int, flag: Bool) -> Result[Int, Str]:\n",
            "    let mut local = seed\n",
            "    defer std.io.print[Int] :: 100 :: call\n",
            "    while local > 0:\n",
            "        let scratch = local\n",
            "        local -= 1\n",
            "    [scratch, cleanup]#cleanup\n",
            "    let value = (maybe :: flag :: call) :: :: ?\n",
            "    return Result.Ok[Int, Str] :: value :: call\n",
            "[seed, cleanup]#cleanup\n",
            "fn main() -> Int:\n",
            "    let result = run :: 2, true :: call\n",
            "    std.io.print[Bool] :: (result :: :: is_err) :: call\n",
            "    return 0\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let graph = load_workspace_graph(&dir).expect("workspace graph should load");
    let checked = check_workspace_graph(&graph).expect("workspace should check");
    let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
        .expect("fingerprints should compute");
    let order = plan_workspace(&graph).expect("workspace order should plan");
    let statuses =
        plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
    execute_build(&graph, &statuses).expect("build should execute");

    let artifact_path = graph.root_dir.join(
        &statuses
            .iter()
            .find(|status| status.member == "runtime_page_rollups")
            .expect("app artifact status should exist")
            .artifact_rel_path,
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(
        host.stdout,
        vec![
            "1".to_string(),
            "2".to_string(),
            "100".to_string(),
            "true".to_string(),
        ]
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_manual_routine_rollups_run_after_defers() {
    let plan = RuntimePackagePlan {
        package_name: "manual_routine_rollups".to_string(),
        root_module_id: "manual_routine_rollups".to_string(),
        direct_deps: Vec::new(),
        runtime_requirements: Vec::new(),
        module_aliases: BTreeMap::new(),
        entrypoints: vec![RuntimeEntrypointPlan {
            module_id: "manual_routine_rollups".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            is_async: false,
            exported: true,
            routine_index: 1,
        }],
        routines: vec![
            RuntimeRoutinePlan {
                module_id: "manual_routine_rollups".to_string(),
                symbol_name: "run".to_string(),
                symbol_kind: "fn".to_string(),
                exported: false,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: vec![RuntimeParamPlan {
                    mode: Some("read".to_string()),
                    name: "seed".to_string(),
                    ty: "Int".to_string(),
                }],
                signature_row: "fn run(read seed: Int) -> Result[Int, Str]:".to_string(),
                intrinsic_impl: None,
                foreword_rows: Vec::new(),
                rollup_rows: vec!["cleanup:seed:std.io.print".to_string()],
                rollups: vec![ParsedPageRollup {
                    kind: "cleanup".to_string(),
                    subject: "seed".to_string(),
                    handler_path: vec!["std".to_string(), "io".to_string(), "print".to_string()],
                }],
                statements: vec![
                    ParsedStmt::Defer(ParsedExpr::Phrase {
                        subject: Box::new(ParsedExpr::Path(vec![
                            "std".to_string(),
                            "io".to_string(),
                            "print".to_string(),
                        ])),
                        args: vec![ParsedPhraseArg {
                            name: None,
                            value: ParsedExpr::Int(100),
                        }],
                        qualifier_kind: ParsedPhraseQualifierKind::Call,
                        qualifier: "call".to_string(),
                        attached: Vec::new(),
                    }),
                    ParsedStmt::Expr {
                        expr: ParsedExpr::Phrase {
                            subject: Box::new(ParsedExpr::Phrase {
                                subject: Box::new(ParsedExpr::Path(vec![
                                    "Result".to_string(),
                                    "Err".to_string(),
                                ])),
                                args: vec![ParsedPhraseArg {
                                    name: None,
                                    value: ParsedExpr::Str("bad".to_string()),
                                }],
                                qualifier_kind: ParsedPhraseQualifierKind::Call,
                                qualifier: "call".to_string(),
                                attached: Vec::new(),
                            }),
                            args: Vec::new(),
                            qualifier_kind: ParsedPhraseQualifierKind::Try,
                            qualifier: "?".to_string(),
                            attached: Vec::new(),
                        },
                        rollups: Vec::new(),
                    },
                ],
            },
            RuntimeRoutinePlan {
                module_id: "manual_routine_rollups".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: Vec::new(),
                signature_row: "fn main() -> Int:".to_string(),
                intrinsic_impl: None,
                foreword_rows: Vec::new(),
                rollup_rows: Vec::new(),
                rollups: Vec::new(),
                statements: vec![
                    ParsedStmt::Expr {
                        expr: ParsedExpr::Phrase {
                            subject: Box::new(ParsedExpr::Path(vec!["run".to_string()])),
                            args: vec![ParsedPhraseArg {
                                name: None,
                                value: ParsedExpr::Int(2),
                            }],
                            qualifier_kind: ParsedPhraseQualifierKind::Call,
                            qualifier: "call".to_string(),
                            attached: Vec::new(),
                        },
                        rollups: Vec::new(),
                    },
                    ParsedStmt::Return(Some(ParsedExpr::Int(0))),
                ],
            },
        ],
    };
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");
    assert_eq!(code, 0);
    assert_eq!(host.stdout, vec!["100".to_string(), "2".to_string()]);
}

#[test]
fn execute_main_runs_counter_style_workspace_artifact() {
    let dir = temp_workspace_dir("counter");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_counter\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.io\n",
            "use std.io as io\n",
            "fn main() -> Int:\n",
            "    let mut i = 0\n",
            "    while i < 3:\n",
            "        io.print[Int] :: i :: call\n",
            "        i += 1\n",
            "    return 0\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let graph = load_workspace_graph(&dir).expect("workspace graph should load");
    let checked = check_workspace_graph(&graph).expect("workspace should check");
    let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
        .expect("fingerprints should compute");
    let order = plan_workspace(&graph).expect("workspace order should plan");
    let statuses =
        plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
    execute_build(&graph, &statuses).expect("build should execute");

    let artifact_path = graph.root_dir.join(
        &statuses
            .iter()
            .find(|status| status.member == "runtime_counter")
            .expect("app artifact status should exist")
            .artifact_rel_path,
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(
        host.stdout,
        vec!["0".to_string(), "1".to_string(), "2".to_string()]
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_routine_calls_with_std_args() {
    let dir = temp_workspace_dir("args");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_args\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.args\n",
            "import std.io\n",
            "fn add_one(value: Int) -> Int:\n",
            "    return value + 1\n",
            "fn main() -> Int:\n",
            "    let argc = std.args.count :: :: call\n",
            "    let total = add_one :: argc :: call\n",
            "    std.io.print[Int] :: total :: call\n",
            "    if argc > 0:\n",
            "        let first = std.args.get :: 0 :: call\n",
            "        std.io.print[Str] :: first :: call\n",
            "    return 0\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let graph = load_workspace_graph(&dir).expect("workspace graph should load");
    let checked = check_workspace_graph(&graph).expect("workspace should check");
    let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
        .expect("fingerprints should compute");
    let order = plan_workspace(&graph).expect("workspace order should plan");
    let statuses =
        plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
    execute_build(&graph, &statuses).expect("build should execute");

    let artifact_path = graph.root_dir.join(
        &statuses
            .iter()
            .find(|status| status.member == "runtime_args")
            .expect("app artifact status should exist")
            .artifact_rel_path,
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost {
        args: vec!["alpha.arc".to_string(), "beta.arc".to_string()],
        ..BufferedHost::default()
    };
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(host.stdout, vec!["3".to_string(), "alpha.arc".to_string()]);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_linked_std_text_routine() {
    let dir = temp_workspace_dir("std_text");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_std_text\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.io\n",
            "import std.text\n",
            "fn main() -> Int:\n",
            "    std.io.print[Int] :: (std.text.find :: \"abc\", 0, \"b\" :: call) :: call\n",
            "    return 0\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let graph = load_workspace_graph(&dir).expect("workspace graph should load");
    let checked = check_workspace_graph(&graph).expect("workspace should check");
    let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
        .expect("fingerprints should compute");
    let order = plan_workspace(&graph).expect("workspace order should plan");
    let statuses =
        plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
    execute_build(&graph, &statuses).expect("build should execute");

    let artifact_path = graph.root_dir.join(
        &statuses
            .iter()
            .find(|status| status.member == "runtime_std_text")
            .expect("app artifact status should exist")
            .artifact_rel_path,
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(host.stdout, vec!["1".to_string()]);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_linked_std_array_routines() {
    let dir = temp_workspace_dir("std_array");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_std_array\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.collections.array\n",
            "import std.collections.list\n",
            "import std.io\n",
            "fn main() -> Int:\n",
            "    let mut values = std.collections.list.new[Int] :: :: call\n",
            "    values :: 4 :: push\n",
            "    values :: 9 :: push\n",
            "    let arr = std.collections.array.from_list[Int] :: values :: call\n",
            "    let mut sum = 0\n",
            "    for value in arr:\n",
            "        sum += value\n",
            "    std.io.print[Int] :: (arr :: :: len) :: call\n",
            "    std.io.print[Int] :: sum :: call\n",
            "    return 0\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let graph = load_workspace_graph(&dir).expect("workspace graph should load");
    let checked = check_workspace_graph(&graph).expect("workspace should check");
    let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
        .expect("fingerprints should compute");
    let order = plan_workspace(&graph).expect("workspace order should plan");
    let statuses =
        plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
    execute_build(&graph, &statuses).expect("build should execute");

    let artifact_path = graph.root_dir.join(
        &statuses
            .iter()
            .find(|status| status.member == "runtime_std_array")
            .expect("app artifact status should exist")
            .artifact_rel_path,
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(host.stdout, vec!["2".to_string(), "13".to_string()]);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_linked_std_iter_and_set_routines() {
    let dir = temp_workspace_dir("std_iter_set");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_std_iter_set\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.collections.set\n",
            "import std.io\n",
            "import std.iter\n",
            "fn main() -> Int:\n",
            "    let mut it = std.iter.range :: 2, 5 :: call\n",
            "    std.io.print[Int] :: (std.iter.count[std.iter.RangeIter] :: it :: call) :: call\n",
            "    let mut xs = std.collections.set.new[Int] :: :: call\n",
            "    std.io.print[Bool] :: (xs :: 7 :: insert) :: call\n",
            "    std.io.print[Bool] :: (xs :: 7 :: insert) :: call\n",
            "    std.io.print[Bool] :: (xs :: 7 :: has) :: call\n",
            "    std.io.print[Int] :: (xs :: :: len) :: call\n",
            "    std.io.print[Bool] :: (xs :: 7 :: remove) :: call\n",
            "    std.io.print[Int] :: (xs :: :: len) :: call\n",
            "    return 0\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let graph = load_workspace_graph(&dir).expect("workspace graph should load");
    let checked = check_workspace_graph(&graph).expect("workspace should check");
    let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
        .expect("fingerprints should compute");
    let order = plan_workspace(&graph).expect("workspace order should plan");
    let statuses =
        plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
    execute_build(&graph, &statuses).expect("build should execute");

    let artifact_path = graph.root_dir.join(
        &statuses
            .iter()
            .find(|status| status.member == "runtime_std_iter_set")
            .expect("app artifact status should exist")
            .artifact_rel_path,
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(
        host.stdout,
        vec![
            "3".to_string(),
            "true".to_string(),
            "false".to_string(),
            "true".to_string(),
            "1".to_string(),
            "true".to_string(),
            "0".to_string(),
        ]
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_linked_std_config_routines() {
    let dir = temp_workspace_dir("std_config");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_std_config\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.config\n",
            "import std.io\n",
            "fn main() -> Int:\n",
            "    let text = \"name = \\\"Arcana\\\"\\n[deps]\\nfoo = { path = \\\"../foo\\\" }\\n[settings]\\nmode = \\\"dev\\\"\\n\"\n",
            "    let parsed = std.config.parse_document :: text :: call\n",
            "    if parsed :: :: is_err:\n",
            "        std.io.print[Str] :: (parsed :: \"parse error\" :: unwrap_or) :: call\n",
            "        return 1\n",
            "    let doc = parsed :: (std.config.empty_document :: :: call) :: unwrap_or\n",
            "    std.io.print[Bool] :: (doc :: \"name\" :: root_has_key) :: call\n",
            "    std.io.print[Bool] :: (doc :: \"settings\" :: has_section) :: call\n",
            "    std.io.print[Str] :: ((doc :: \"name\", \"config field\" :: root_required_string) :: \"missing\" :: unwrap_or) :: call\n",
            "    std.io.print[Str] :: ((doc :: \"settings\", \"mode\", \"settings field\" :: section_required) :: \"missing\" :: unwrap_or) :: call\n",
            "    std.io.print[Str] :: ((doc :: \"deps\", (\"foo\", \"path\"), \"dependency entry\" :: section_inline_table_string_field) :: \"missing\" :: unwrap_or) :: call\n",
            "    std.io.print[Int] :: ((doc :: \"settings\" :: entries_in_section) :: :: len) :: call\n",
            "    return 0\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let graph = load_workspace_graph(&dir).expect("workspace graph should load");
    let checked = check_workspace_graph(&graph).expect("workspace should check");
    let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
        .expect("fingerprints should compute");
    let order = plan_workspace(&graph).expect("workspace order should plan");
    let statuses =
        plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
    execute_build(&graph, &statuses).expect("build should execute");

    let artifact_path = graph.root_dir.join(
        &statuses
            .iter()
            .find(|status| status.member == "runtime_std_config")
            .expect("app artifact status should exist")
            .artifact_rel_path,
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(
        host.stdout,
        vec![
            "true".to_string(),
            "true".to_string(),
            "Arcana".to_string(),
            "dev".to_string(),
            "../foo".to_string(),
            "1".to_string(),
        ]
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_linked_std_manifest_routines() {
    let dir = temp_workspace_dir("std_manifest");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_std_manifest\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.collections.list\n",
            "import std.io\n",
            "import std.manifest\n",
            "fn main() -> Int:\n",
            "    let book = \"name = \\\"demo\\\"\\nkind = \\\"app\\\"\\n[workspace]\\nmembers = [\\\"game\\\", \\\"tools\\\"]\\n[deps]\\nfoo = { path = \\\"../foo\\\" }\\n\"\n",
            "    let parsed_book = std.manifest.parse_book :: book :: call\n",
            "    if parsed_book :: :: is_err:\n",
            "        std.io.print[Str] :: (parsed_book :: \"book parse error\" :: unwrap_or) :: call\n",
            "        return 1\n",
            "    let book_manifest = parsed_book :: (std.manifest.BookManifest :: state = (std.manifest.BookState :: name = \"\", kind = \"\", workspace_member_names = (std.collections.list.new[Str] :: :: call) :: call), dependency_paths = (std.collections.list.new[std.manifest.NameValue] :: :: call) :: call) :: unwrap_or\n",
            "    let members = book_manifest :: :: workspace_members\n",
            "    std.io.print[Int] :: ((members :: (std.collections.list.new[Str] :: :: call) :: unwrap_or) :: :: len) :: call\n",
            "    std.io.print[Str] :: ((book_manifest :: \"foo\" :: dep_path) :: \"missing\" :: unwrap_or) :: call\n",
            "    let lock = \"version = 1\\nworkspace = \\\"demo\\\"\\norder = [\\\"game\\\", \\\"tools\\\"]\\n[deps]\\ngame = [\\\"foo\\\", \\\"bar\\\"]\\n[paths]\\ngame = \\\"grimoires/owned/app/game\\\"\\n[fingerprints]\\ngame = \\\"fp1\\\"\\n[api_fingerprints]\\ngame = \\\"api1\\\"\\n[artifacts]\\ngame = \\\"build/app.artifact.toml\\\"\\n[kinds]\\ngame = \\\"app\\\"\\n[formats]\\ngame = \\\"arcana-aot-v2\\\"\\n\"\n",
            "    let parsed_lock = std.manifest.parse_lock_v1 :: lock :: call\n",
            "    if parsed_lock :: :: is_err:\n",
            "        std.io.print[Str] :: (parsed_lock :: \"lock parse error\" :: unwrap_or) :: call\n",
            "        return 1\n",
            "    let empty_metadata = std.manifest.LockMetadata :: version = 0, workspace = \"\", ordered_members = (std.collections.list.new[Str] :: :: call) :: call\n",
            "    let empty_deps = std.manifest.LockDependencyTables :: dependency_lists = (std.collections.list.new[std.manifest.NameList] :: :: call), path_entries = (std.collections.list.new[std.manifest.NameValue] :: :: call), fingerprint_entries = (std.collections.list.new[std.manifest.NameValue] :: :: call) :: call\n",
            "    let empty_lookup = std.manifest.LockLookupTables :: dependencies = empty_deps, api_fingerprint_entries = (std.collections.list.new[std.manifest.NameValue] :: :: call) :: call\n",
            "    let empty_output = std.manifest.LockOutputTables :: artifact_entries = (std.collections.list.new[std.manifest.NameValue] :: :: call), kind_entries = (std.collections.list.new[std.manifest.NameValue] :: :: call), format_entries = (std.collections.list.new[std.manifest.NameValue] :: :: call) :: call\n",
            "    let lock_manifest = parsed_lock :: (std.manifest.LockManifestV1 :: metadata = empty_metadata, lookup_tables = empty_lookup, output_tables = empty_output :: call) :: unwrap_or\n",
            "    let deps = lock_manifest :: \"game\" :: deps_for\n",
            "    std.io.print[Int] :: ((deps :: (std.collections.list.new[Str] :: :: call) :: unwrap_or) :: :: len) :: call\n",
            "    std.io.print[Str] :: ((lock_manifest :: \"game\" :: path_for) :: \"missing\" :: unwrap_or) :: call\n",
            "    std.io.print[Str] :: ((lock_manifest :: \"game\" :: kind_for) :: \"missing\" :: unwrap_or) :: call\n",
            "    std.io.print[Str] :: ((lock_manifest :: \"game\" :: format_for) :: \"missing\" :: unwrap_or) :: call\n",
            "    return 0\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let graph = load_workspace_graph(&dir).expect("workspace graph should load");
    let checked = check_workspace_graph(&graph).expect("workspace should check");
    let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
        .expect("fingerprints should compute");
    let order = plan_workspace(&graph).expect("workspace order should plan");
    let statuses =
        plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
    execute_build(&graph, &statuses).expect("build should execute");

    let artifact_path = graph.root_dir.join(
        &statuses
            .iter()
            .find(|status| status.member == "runtime_std_manifest")
            .expect("app artifact status should exist")
            .artifact_rel_path,
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(
        host.stdout,
        vec![
            "2".to_string(),
            "../foo".to_string(),
            "2".to_string(),
            "grimoires/owned/app/game".to_string(),
            "app".to_string(),
            "arcana-aot-v2".to_string(),
        ]
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_linked_std_concurrent_routines() {
    let dir = temp_workspace_dir("std_concurrent");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_std_concurrent\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.concurrent\n",
            "import std.io\n",
            "fn main() -> Int:\n",
            "    let ch = std.concurrent.channel[Int] :: 2 :: call\n",
            "    ch :: 4 :: send\n",
            "    ch :: 9 :: send\n",
            "    std.io.print[Int] :: (ch :: :: recv) :: call\n",
            "    std.io.print[Int] :: (ch :: :: recv) :: call\n",
            "    let m = std.concurrent.mutex[Int] :: 11 :: call\n",
            "    std.io.print[Int] :: (m :: :: pull) :: call\n",
            "    m :: 15 :: put\n",
            "    std.io.print[Int] :: (m :: :: pull) :: call\n",
            "    let ai = std.concurrent.atomic_int :: 7 :: call\n",
            "    std.io.print[Int] :: (ai :: :: load) :: call\n",
            "    ai :: 5 :: add\n",
            "    ai :: 3 :: sub\n",
            "    std.io.print[Int] :: (ai :: :: load) :: call\n",
            "    std.io.print[Int] :: (ai :: 20 :: swap) :: call\n",
            "    std.io.print[Int] :: (ai :: :: load) :: call\n",
            "    let ab = std.concurrent.atomic_bool :: true :: call\n",
            "    std.io.print[Bool] :: (ab :: :: load) :: call\n",
            "    std.io.print[Bool] :: (ab :: false :: swap) :: call\n",
            "    std.io.print[Bool] :: (ab :: :: load) :: call\n",
            "    std.io.print[Int] :: (std.concurrent.thread_id :: :: call) :: call\n",
            "    std.concurrent.sleep :: 5 :: call\n",
            "    return 0\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let graph = load_workspace_graph(&dir).expect("workspace graph should load");
    let checked = check_workspace_graph(&graph).expect("workspace should check");
    let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
        .expect("fingerprints should compute");
    let order = plan_workspace(&graph).expect("workspace order should plan");
    let statuses =
        plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
    execute_build(&graph, &statuses).expect("build should execute");

    let artifact_path = graph.root_dir.join(
        &statuses
            .iter()
            .find(|status| status.member == "runtime_std_concurrent")
            .expect("app artifact status should exist")
            .artifact_rel_path,
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(
        host.stdout,
        vec![
            "4".to_string(),
            "9".to_string(),
            "11".to_string(),
            "15".to_string(),
            "7".to_string(),
            "9".to_string(),
            "9".to_string(),
            "20".to_string(),
            "true".to_string(),
            "true".to_string(),
            "false".to_string(),
            "0".to_string(),
        ]
    );
    assert_eq!(host.sleep_log_ms, vec![5]);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_linked_std_memory_routines() {
    let dir = temp_workspace_dir("std_memory");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_std_memory\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.io\n",
            "import std.memory\n",
            "record Item:\n",
            "    value: Int\n",
            "fn main() -> Int:\n",
            "    let mut arena_store = std.memory.new[Item] :: 4 :: call\n",
            "    let arena_id = arena: arena_store :> value = 7 <: Item\n",
            "    std.io.print[Int] :: (arena_store :: :: len) :: call\n",
            "    std.io.print[Bool] :: (arena_store :: arena_id :: has) :: call\n",
            "    let arena_item = arena_store :: arena_id :: get\n",
            "    std.io.print[Int] :: arena_item.value :: call\n",
            "    arena_store :: arena_id, (Item :: value = 9 :: call) :: set\n",
            "    let arena_item2 = arena_store :: arena_id :: get\n",
            "    std.io.print[Int] :: arena_item2.value :: call\n",
            "    std.io.print[Bool] :: (arena_store :: arena_id :: remove) :: call\n",
            "    std.io.print[Bool] :: (arena_store :: arena_id :: has) :: call\n",
            "    let mut frame_store = std.memory.frame_new[Item] :: 2 :: call\n",
            "    let frame_id = frame: frame_store :> value = 11 <: Item\n",
            "    let frame_item = frame_store :: frame_id :: get\n",
            "    std.io.print[Int] :: frame_item.value :: call\n",
            "    frame_store :: :: reset\n",
            "    std.io.print[Bool] :: (frame_store :: frame_id :: has) :: call\n",
            "    let mut pool_store = std.memory.pool_new[Item] :: 2 :: call\n",
            "    let pool_a = pool: pool_store :> value = 21 <: Item\n",
            "    let pool_item = pool_store :: pool_a :: get\n",
            "    std.io.print[Int] :: pool_item.value :: call\n",
            "    std.io.print[Bool] :: (pool_store :: pool_a :: remove) :: call\n",
            "    let pool_b = pool: pool_store :> value = 34 <: Item\n",
            "    std.io.print[Bool] :: (pool_store :: pool_a :: has) :: call\n",
            "    let pool_item2 = pool_store :: pool_b :: get\n",
            "    std.io.print[Int] :: pool_item2.value :: call\n",
            "    return 0\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let graph = load_workspace_graph(&dir).expect("workspace graph should load");
    let checked = check_workspace_graph(&graph).expect("workspace should check");
    let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
        .expect("fingerprints should compute");
    let order = plan_workspace(&graph).expect("workspace order should plan");
    let statuses =
        plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
    execute_build(&graph, &statuses).expect("build should execute");

    let artifact_path = graph.root_dir.join(
        &statuses
            .iter()
            .find(|status| status.member == "runtime_std_memory")
            .expect("app artifact status should exist")
            .artifact_rel_path,
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(
        host.stdout,
        vec![
            "1".to_string(),
            "true".to_string(),
            "7".to_string(),
            "9".to_string(),
            "true".to_string(),
            "false".to_string(),
            "11".to_string(),
            "false".to_string(),
            "21".to_string(),
            "true".to_string(),
            "false".to_string(),
            "34".to_string(),
        ]
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_linked_std_memory_borrow_routines() {
    let dir = temp_workspace_dir("std_memory_borrow");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_std_memory_borrow\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.io\n",
            "import std.memory\n",
            "record Counter:\n",
            "    value: Int\n",
            "fn bump(edit counter: Counter):\n",
            "    counter.value += 1\n",
            "fn main() -> Int:\n",
            "    let mut arena_store = std.memory.new[Counter] :: 1 :: call\n",
            "    let counter_id = arena: arena_store :> value = 9 <: Counter\n",
            "    let current = arena_store :: counter_id :: borrow_read\n",
            "    let current_value = *current\n",
            "    std.io.print[Int] :: current_value.value :: call\n",
            "    let mut slot = arena_store :: counter_id :: borrow_edit\n",
            "    bump :: slot :: call\n",
            "    let updated = arena_store :: counter_id :: get\n",
            "    std.io.print[Int] :: updated.value :: call\n",
            "    return 0\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let graph = load_workspace_graph(&dir).expect("workspace graph should load");
    let checked = check_workspace_graph(&graph).expect("workspace should check");
    let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
        .expect("fingerprints should compute");
    let order = plan_workspace(&graph).expect("workspace order should plan");
    let statuses =
        plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
    execute_build(&graph, &statuses).expect("build should execute");

    let artifact_path = graph.root_dir.join(
        &statuses
            .iter()
            .find(|status| status.member == "runtime_std_memory_borrow")
            .expect("app artifact status should exist")
            .artifact_rel_path,
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(host.stdout, vec!["9".to_string(), "10".to_string()]);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_memory_phrase_attachment_routines() {
    let dir = temp_workspace_dir("memory_phrase_attachments");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_memory_phrase_attachments\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.io\n",
            "import std.memory\n",
            "record Counter:\n",
            "    value: Int\n",
            "fn make_counter(value: Int, bonus: Int) -> Counter:\n",
            "    std.io.print[Int] :: bonus :: call\n",
            "    return Counter :: value = value + bonus :: call\n",
            "fn main() -> Int:\n",
            "    let mut arena_store = std.memory.new[Counter] :: 2 :: call\n",
            "    arena: arena_store :> 9 <: make_counter\n",
            "        bonus = 4\n",
            "    std.io.print[Int] :: (arena_store :: :: len) :: call\n",
            "    let id = arena: arena_store :> value = 1 <: Counter\n",
            "    let item = arena_store :: id :: get\n",
            "    std.io.print[Int] :: item.value :: call\n",
            "    return 0\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let graph = load_workspace_graph(&dir).expect("workspace graph should load");
    let checked = check_workspace_graph(&graph).expect("workspace should check");
    let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
        .expect("fingerprints should compute");
    let order = plan_workspace(&graph).expect("workspace order should plan");
    let statuses =
        plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
    execute_build(&graph, &statuses).expect("build should execute");

    let artifact_path = graph.root_dir.join(
        &statuses
            .iter()
            .find(|status| status.member == "runtime_memory_phrase_attachments")
            .expect("app artifact status should exist")
            .artifact_rel_path,
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(
        host.stdout,
        vec!["4".to_string(), "1".to_string(), "1".to_string()]
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_local_borrow_and_deref_routines() {
    let dir = temp_workspace_dir("local_borrow");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_local_borrow\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.io\n",
            "fn main() -> Int:\n",
            "    let local_x = 1\n",
            "    let mut local_y = 2\n",
            "    let x_ref = &local_x\n",
            "    let y_ref = &mut local_y\n",
            "    let sum = *x_ref + *y_ref\n",
            "    std.io.print[Int] :: sum :: call\n",
            "    return 0\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let graph = load_workspace_graph(&dir).expect("workspace graph should load");
    let checked = check_workspace_graph(&graph).expect("workspace should check");
    let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
        .expect("fingerprints should compute");
    let order = plan_workspace(&graph).expect("workspace order should plan");
    let statuses =
        plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
    execute_build(&graph, &statuses).expect("build should execute");

    let artifact_path = graph.root_dir.join(
        &statuses
            .iter()
            .find(|status| status.member == "runtime_local_borrow")
            .expect("app artifact status should exist")
            .artifact_rel_path,
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(host.stdout, vec!["3".to_string()]);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_linked_std_concurrent_task_thread_routines() {
    let dir = temp_workspace_dir("std_concurrent_async");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_std_concurrent_async\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.io\n",
            "async fn worker(value: Int) -> Int:\n",
            "    return value + 1\n",
            "fn helper(value: Int) -> Int:\n",
            "    return value * 2\n",
            "fn main() -> Int:\n",
            "    let task = weave worker :: 41 :: call\n",
            "    let thread = split helper :: 7 :: call\n",
            "    std.io.print[Bool] :: (task :: :: done) :: call\n",
            "    std.io.print[Bool] :: (thread :: :: done) :: call\n",
            "    std.io.print[Int] :: (task :: :: join) :: call\n",
            "    std.io.print[Int] :: (thread :: :: join) :: call\n",
            "    let awaited_task = task >> await\n",
            "    let awaited_thread = thread >> await\n",
            "    std.io.print[Int] :: awaited_task :: call\n",
            "    std.io.print[Int] :: awaited_thread :: call\n",
            "    return 0\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let graph = load_workspace_graph(&dir).expect("workspace graph should load");
    let checked = check_workspace_graph(&graph).expect("workspace should check");
    let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
        .expect("fingerprints should compute");
    let order = plan_workspace(&graph).expect("workspace order should plan");
    let statuses =
        plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
    execute_build(&graph, &statuses).expect("build should execute");

    let artifact_path = graph.root_dir.join(
        &statuses
            .iter()
            .find(|status| status.member == "runtime_std_concurrent_async")
            .expect("app artifact status should exist")
            .artifact_rel_path,
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(
        host.stdout,
        vec![
            "false".to_string(),
            "false".to_string(),
            "42".to_string(),
            "14".to_string(),
            "42".to_string(),
            "14".to_string(),
        ]
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_async_main_entrypoint() {
    let dir = temp_workspace_dir("async_main");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_async_main\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.io\n",
            "async fn compute() -> Int:\n",
            "    return 5\n",
            "async fn main() -> Int:\n",
            "    std.io.print[Int] :: (compute :: :: call) :: call\n",
            "    return 7\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let graph = load_workspace_graph(&dir).expect("workspace graph should load");
    let checked = check_workspace_graph(&graph).expect("workspace should check");
    let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
        .expect("fingerprints should compute");
    let order = plan_workspace(&graph).expect("workspace order should plan");
    let statuses =
        plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
    execute_build(&graph, &statuses).expect("build should execute");

    let artifact_path = graph.root_dir.join(
        &statuses
            .iter()
            .find(|status| status.member == "runtime_async_main")
            .expect("app artifact status should exist")
            .artifact_rel_path,
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 7);
    assert_eq!(host.stdout, vec!["5".to_string()]);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_defers_non_call_spawned_values_until_join() {
    let dir = temp_workspace_dir("spawned_values_pending");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_spawned_values_pending\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.io\n",
            "fn main() -> Int:\n",
            "    let task = weave 7\n",
            "    let thread = split 8\n",
            "    std.io.print[Bool] :: (task :: :: done) :: call\n",
            "    std.io.print[Bool] :: (thread :: :: done) :: call\n",
            "    std.io.print[Int] :: (task :: :: join) :: call\n",
            "    std.io.print[Int] :: (thread :: :: join) :: call\n",
            "    return 0\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let graph = load_workspace_graph(&dir).expect("workspace graph should load");
    let checked = check_workspace_graph(&graph).expect("workspace should check");
    let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
        .expect("fingerprints should compute");
    let order = plan_workspace(&graph).expect("workspace order should plan");
    let statuses =
        plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
    execute_build(&graph, &statuses).expect("build should execute");

    let artifact_path = graph.root_dir.join(
        &statuses
            .iter()
            .find(|status| status.member == "runtime_spawned_values_pending")
            .expect("app artifact status should exist")
            .artifact_rel_path,
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(
        host.stdout,
        vec![
            "false".to_string(),
            "false".to_string(),
            "7".to_string(),
            "8".to_string(),
        ]
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_split_threads_report_distinct_thread_ids() {
    let dir = temp_workspace_dir("split_thread_id");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_split_thread_id\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.concurrent\n",
            "import std.io\n",
            "fn worker() -> Int:\n",
            "    return std.concurrent.thread_id :: :: call\n",
            "fn main() -> Int:\n",
            "    std.io.print[Int] :: (std.concurrent.thread_id :: :: call) :: call\n",
            "    let thread = split worker :: :: call\n",
            "    std.io.print[Bool] :: (thread :: :: done) :: call\n",
            "    std.io.print[Int] :: (thread :: :: join) :: call\n",
            "    return 0\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let graph = load_workspace_graph(&dir).expect("workspace graph should load");
    let checked = check_workspace_graph(&graph).expect("workspace should check");
    let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
        .expect("fingerprints should compute");
    let order = plan_workspace(&graph).expect("workspace order should plan");
    let statuses =
        plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
    execute_build(&graph, &statuses).expect("build should execute");

    let artifact_path = graph.root_dir.join(
        &statuses
            .iter()
            .find(|status| status.member == "runtime_split_thread_id")
            .expect("app artifact status should exist")
            .artifact_rel_path,
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(
        host.stdout,
        vec!["0".to_string(), "false".to_string(), "1".to_string(),]
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_chain_expressions_with_parallel_fanout() {
    let dir = temp_workspace_dir("chain_runtime");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_chain\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.io\n",
            "fn seed() -> Int:\n",
            "    return 2\n",
            "fn inc(value: Int) -> Int:\n",
            "    return value + 1\n",
            "fn mul(value: Int) -> Int:\n",
            "    return value * 2\n",
            "fn main() -> Int:\n",
            "    let pipeline = forward :=> seed => inc => mul\n",
            "    std.io.print[Int] :: pipeline :: call\n",
            "    let fanout = parallel :=> seed => inc => mul\n",
            "    std.io.print[Int] :: fanout[0] :: call\n",
            "    std.io.print[Int] :: fanout[1] :: call\n",
            "    return 0\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let graph = load_workspace_graph(&dir).expect("workspace graph should load");
    let checked = check_workspace_graph(&graph).expect("workspace should check");
    let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
        .expect("fingerprints should compute");
    let order = plan_workspace(&graph).expect("workspace order should plan");
    let statuses =
        plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
    execute_build(&graph, &statuses).expect("build should execute");

    let artifact_path = graph.root_dir.join(
        &statuses
            .iter()
            .find(|status| status.member == "runtime_chain")
            .expect("app artifact status should exist")
            .artifact_rel_path,
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(
        host.stdout,
        vec!["6".to_string(), "3".to_string(), "4".to_string()]
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_linked_std_host_text_bytes_io_env_routines() {
    let dir = temp_workspace_dir("std_host_misc");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_std_host_misc\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.bytes\n",
            "import std.env\n",
            "import std.io\n",
            "import std.text\n",
            "use std.result.Result\n",
            "fn main() -> Int:\n",
            "    let label = std.env.get_or :: \"ARCANA_LABEL\", \"unset\" :: call\n",
            "    let input = match (std.io.read_line :: :: call):\n",
            "        Result.Ok(value) => value\n",
            "        Result.Err(err) => err\n",
            "    let lines = std.text.split_lines :: \"alpha\\r\\nbeta\\n\" :: call\n",
            "    let bytes = std.bytes.from_str_utf8 :: input :: call\n",
            "    let mid = std.bytes.slice :: bytes, 1, 4 :: call\n",
            "    std.io.flush_stdout :: :: call\n",
            "    std.io.flush_stderr :: :: call\n",
            "    std.io.print[Str] :: label :: call\n",
            "    std.io.print[Bool] :: (std.text.starts_with :: input, \"he\" :: call) :: call\n",
            "    std.io.print[Bool] :: (std.text.ends_with :: input, \"lo\" :: call) :: call\n",
            "    std.io.print[Int] :: (lines :: :: len) :: call\n",
            "    std.io.print[Str] :: (std.text.from_int :: (std.bytes.len :: bytes :: call) :: call) :: call\n",
            "    std.io.print[Int] :: (std.bytes.at :: bytes, 1 :: call) :: call\n",
            "    std.io.print[Str] :: (std.bytes.to_str_utf8 :: mid :: call) :: call\n",
            "    std.io.print[Str] :: (std.bytes.sha256_hex :: bytes :: call) :: call\n",
            "    return 0\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let graph = load_workspace_graph(&dir).expect("workspace graph should load");
    let checked = check_workspace_graph(&graph).expect("workspace should check");
    let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
        .expect("fingerprints should compute");
    let order = plan_workspace(&graph).expect("workspace order should plan");
    let statuses =
        plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
    execute_build(&graph, &statuses).expect("build should execute");

    let artifact_path = graph.root_dir.join(
        &statuses
            .iter()
            .find(|status| status.member == "runtime_std_host_misc")
            .expect("app artifact status should exist")
            .artifact_rel_path,
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost {
        stdin: vec!["hello".to_string()],
        env: std::collections::BTreeMap::from([(
            "ARCANA_LABEL".to_string(),
            "runtime".to_string(),
        )]),
        ..BufferedHost::default()
    };
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(host.stdout_flushes, 1);
    assert_eq!(host.stderr_flushes, 1);
    assert_eq!(
        host.stdout,
        vec![
            "runtime".to_string(),
            "true".to_string(),
            "true".to_string(),
            "2".to_string(),
            "5".to_string(),
            "101".to_string(),
            "ell".to_string(),
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824".to_string(),
        ]
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_linked_std_wrapper_closure_routines() {
    let dir = temp_workspace_dir("std_wrapper_closure");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_std_wrapper_closure\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.bytes\n",
            "import std.collections.array\n",
            "import std.collections.list\n",
            "import std.collections.map\n",
            "import std.collections.set\n",
            "import std.io\n",
            "import std.path\n",
            "import std.text\n",
            "import std.time\n",
            "import std.types.core\n",
            "use std.bytes as bytes\n",
            "use std.collections.array as arrays\n",
            "use std.collections.list as lists\n",
            "use std.collections.map as maps\n",
            "use std.collections.set as sets\n",
            "use std.path as paths\n",
            "use std.text as texts\n",
            "use std.time as times\n",
            "use std.types.core as core\n",
            "use std.result.Result\n",
            "fn unwrap_str(result: Result[Str, Str]) -> Str:\n",
            "    return match result:\n",
            "        Result.Ok(value) => value\n",
            "        Result.Err(err) => err\n",
            "fn unwrap_int(result: Result[Int, Str]) -> Int:\n",
            "    return match result:\n",
            "        Result.Ok(value) => value\n",
            "        Result.Err(_) => -1\n",
            "fn main() -> Int:\n",
            "    let cwd = paths.cwd :: :: call\n",
            "    let file = paths.join :: cwd, \"assets/alpha.txt\" :: call\n",
            "    let weird = paths.join :: cwd, \"assets/../assets/alpha.txt\" :: call\n",
            "    let norm = paths.normalize :: weird :: call\n",
            "    std.io.print[Bool] :: (paths.is_absolute :: norm :: call) :: call\n",
            "    std.io.print[Str] :: (paths.parent :: norm :: call) :: call\n",
            "    std.io.print[Str] :: (paths.file_name :: norm :: call) :: call\n",
            "    std.io.print[Str] :: (paths.ext :: norm :: call) :: call\n",
            "    std.io.print[Str] :: (unwrap_str :: (paths.stem :: norm :: call) :: call) :: call\n",
            "    std.io.print[Str] :: (paths.with_ext :: norm, \"bin\" :: call) :: call\n",
            "    std.io.print[Str] :: (unwrap_str :: (paths.relative_to :: norm, cwd :: call) :: call) :: call\n",
            "    std.io.print[Str] :: (unwrap_str :: (paths.strip_prefix :: norm, cwd :: call) :: call) :: call\n",
            "    std.io.print[Str] :: (paths.file_name :: (unwrap_str :: (paths.canonicalize :: file :: call) :: call) :: call) :: call\n",
            "    let trimmed = texts.trim :: \"  alpha,beta  \" :: call\n",
            "    let parts = texts.split :: trimmed, \",\" :: call\n",
            "    std.io.print[Int] :: (parts :: :: len) :: call\n",
            "    std.io.print[Str] :: (texts.join :: parts, \"+\" :: call) :: call\n",
            "    std.io.print[Str] :: (texts.repeat :: \"ha\", 3 :: call) :: call\n",
            "    std.io.print[Int] :: (unwrap_int :: (texts.to_int :: \"  -42 \" :: call) :: call) :: call\n",
            "    let arc = bytes.from_str_utf8 :: \"arcana\" :: call\n",
            "    let prefix = bytes.from_str_utf8 :: \"arc\" :: call\n",
            "    let can = bytes.from_str_utf8 :: \"can\" :: call\n",
            "    let na = bytes.from_str_utf8 :: \"na\" :: call\n",
            "    std.io.print[Bool] :: (bytes.starts_with :: arc, prefix :: call) :: call\n",
            "    std.io.print[Bool] :: (bytes.ends_with :: arc, na :: call) :: call\n",
            "    std.io.print[Int] :: (bytes.find :: arc, 0, can :: call) :: call\n",
            "    std.io.print[Bool] :: (bytes.contains :: arc, can :: call) :: call\n",
            "    let mut buf = bytes.new_buf :: :: call\n",
            "    std.io.print[Bool] :: ((bytes.buf_push :: buf, 65 :: call) :: :: is_ok) :: call\n",
            "    std.io.print[Int] :: (unwrap_int :: (bytes.buf_extend :: buf, can :: call) :: call) :: call\n",
            "    let combo = bytes.concat :: prefix, (bytes.buf_to_array :: buf :: call) :: call\n",
            "    std.io.print[Str] :: (bytes.to_str_utf8 :: combo :: call) :: call\n",
            "    let pos = core.vec2 :: 3, 4 :: call\n",
            "    let size = core.size2 :: 5, 6 :: call\n",
            "    let rect = core.rect :: pos, size :: call\n",
            "    let color = core.rgb :: 7, 8, 9 :: call\n",
            "    std.io.print[Int] :: (rect.pos.x + rect.size.h) :: call\n",
            "    std.io.print[Int] :: color.g :: call\n",
            "    let start = times.monotonic_now_ms :: :: call\n",
            "    let end = times.monotonic_now_ms :: :: call\n",
            "    let elapsed = times.elapsed_ms :: start, end :: call\n",
            "    times.sleep :: elapsed :: call\n",
            "    times.sleep_ms :: 3 :: call\n",
            "    std.io.print[Int] :: elapsed.value :: call\n",
            "    std.io.print[Int] :: (times.monotonic_now_ns :: :: call) :: call\n",
            "    let arr = arrays.new[Int] :: 3, 4 :: call\n",
            "    let arr_list = arr :: :: to_list\n",
            "    std.io.print[Int] :: (arr_list :: :: len) :: call\n",
            "    let mut xs = lists.new[Int] :: :: call\n",
            "    xs :: arr :: extend_array\n",
            "    let mut ys = lists.new[Int] :: :: call\n",
            "    ys :: 9 :: push\n",
            "    xs :: ys :: extend_list\n",
            "    std.io.print[Int] :: (xs :: :: len) :: call\n",
            "    ys :: :: clear\n",
            "    std.io.print[Bool] :: (ys :: :: is_empty) :: call\n",
            "    let pop_pair = xs :: 0 :: try_pop_or\n",
            "    std.io.print[Bool] :: pop_pair.0 :: call\n",
            "    std.io.print[Int] :: pop_pair.1 :: call\n",
            "    let mut mapping = maps.new[Str, Int] :: :: call\n",
            "    mapping :: \"a\", 1 :: set\n",
            "    mapping :: \"b\", 2 :: set\n",
            "    std.io.print[Int] :: ((maps.keys[Str, Int] :: mapping :: call) :: :: len) :: call\n",
            "    std.io.print[Int] :: ((maps.values[Str, Int] :: mapping :: call) :: :: len) :: call\n",
            "    std.io.print[Int] :: ((maps.items[Str, Int] :: mapping :: call) :: :: len) :: call\n",
            "    let mut set = sets.new[Int] :: :: call\n",
            "    set :: 5 :: insert\n",
            "    set :: 6 :: insert\n",
            "    std.io.print[Int] :: ((sets.items[Int] :: set :: call) :: :: len) :: call\n",
            "    return 0\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let graph = load_workspace_graph(&dir).expect("workspace graph should load");
    let checked = check_workspace_graph(&graph).expect("workspace should check");
    let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
        .expect("fingerprints should compute");
    let order = plan_workspace(&graph).expect("workspace order should plan");
    let statuses =
        plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
    execute_build(&graph, &statuses).expect("build should execute");

    let fixture_root = dir.join("fixture");
    let assets_dir = fixture_root.join("assets");
    fs::create_dir_all(&assets_dir).expect("fixture assets dir should exist");
    let asset_path = assets_dir.join("alpha.txt");
    fs::write(&asset_path, "closure").expect("fixture asset should write");

    let artifact_path = graph.root_dir.join(
        &statuses
            .iter()
            .find(|status| status.member == "runtime_std_wrapper_closure")
            .expect("app artifact status should exist")
            .artifact_rel_path,
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let cwd = fixture_root.to_string_lossy().replace('\\', "/");
    let parent = assets_dir.to_string_lossy().replace('\\', "/");
    let with_ext = assets_dir
        .join("alpha.bin")
        .to_string_lossy()
        .replace('\\', "/");
    let mut host = BufferedHost {
        cwd: cwd.clone(),
        sandbox_root: cwd,
        monotonic_now_ms: 100,
        monotonic_now_ns: 1000,
        monotonic_step_ms: 5,
        monotonic_step_ns: 11,
        ..BufferedHost::default()
    };
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(
        host.stdout,
        vec![
            "true".to_string(),
            parent,
            "alpha.txt".to_string(),
            "txt".to_string(),
            "alpha".to_string(),
            with_ext,
            "assets/alpha.txt".to_string(),
            "assets/alpha.txt".to_string(),
            "alpha.txt".to_string(),
            "2".to_string(),
            "alpha+beta".to_string(),
            "hahaha".to_string(),
            "-42".to_string(),
            "true".to_string(),
            "true".to_string(),
            "2".to_string(),
            "true".to_string(),
            "true".to_string(),
            "3".to_string(),
            "arcAcan".to_string(),
            "9".to_string(),
            "8".to_string(),
            "5".to_string(),
            "1000".to_string(),
            "3".to_string(),
            "4".to_string(),
            "true".to_string(),
            "true".to_string(),
            "9".to_string(),
            "2".to_string(),
            "2".to_string(),
            "2".to_string(),
            "2".to_string(),
        ]
    );
    assert_eq!(host.sleep_log_ms, vec![5, 3]);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_linked_std_fs_bytes_routines() {
    let dir = temp_workspace_dir("std_fs_bytes");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_std_fs_bytes\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.bytes\n",
            "import std.fs\n",
            "import std.io\n",
            "import std.path\n",
            "use std.result.Result\n",
            "fn unwrap_unit(result: Result[Unit, Str]) -> Bool:\n",
            "    return match result:\n",
            "        Result.Ok(_) => true\n",
            "        Result.Err(_) => false\n",
            "fn unwrap_bytes(result: Result[Array[Int], Str]) -> Array[Int]:\n",
            "    return match result:\n",
            "        Result.Ok(value) => value\n",
            "        Result.Err(_) => std.bytes.from_str_utf8 :: \"\" :: call\n",
            "fn unwrap_int(result: Result[Int, Str]) -> Int:\n",
            "    return match result:\n",
            "        Result.Ok(value) => value\n",
            "        Result.Err(_) => -1\n",
            "fn main() -> Int:\n",
            "    let root = std.path.cwd :: :: call\n",
            "    let data_dir = std.path.join :: root, \"data\" :: call\n",
            "    let nested_dir = std.path.join :: data_dir, \"nested\" :: call\n",
            "    let empty_dir = std.path.join :: root, \"empty\" :: call\n",
            "    let source = std.path.join :: data_dir, \"payload.bin\" :: call\n",
            "    let copied = std.path.join :: nested_dir, \"copied.bin\" :: call\n",
            "    let moved = std.path.join :: root, \"moved.bin\" :: call\n",
            "    if not (unwrap_unit :: (std.fs.create_dir :: empty_dir :: call) :: call):\n",
            "        return 1\n",
            "    if not (unwrap_unit :: (std.fs.remove_dir :: empty_dir :: call) :: call):\n",
            "        return 2\n",
            "    if not (unwrap_unit :: (std.fs.create_dir :: data_dir :: call) :: call):\n",
            "        return 3\n",
            "    if not (unwrap_unit :: (std.fs.mkdir_all :: nested_dir :: call) :: call):\n",
            "        return 4\n",
            "    let payload = std.bytes.from_str_utf8 :: \"arc\" :: call\n",
            "    if not (unwrap_unit :: (std.fs.write_bytes :: source, payload :: call) :: call):\n",
            "        return 5\n",
            "    if not (unwrap_unit :: (std.fs.copy_file :: source, copied :: call) :: call):\n",
            "        return 6\n",
            "    if not (unwrap_unit :: (std.fs.rename :: copied, moved :: call) :: call):\n",
            "        return 7\n",
            "    let read_back = unwrap_bytes :: (std.fs.read_bytes :: moved :: call) :: call\n",
            "    let size = unwrap_int :: (std.fs.file_size :: moved :: call) :: call\n",
            "    let modified = unwrap_int :: (std.fs.modified_unix_ms :: moved :: call) :: call\n",
            "    std.io.print[Bool] :: (std.fs.exists :: source :: call) :: call\n",
            "    std.io.print[Str] :: (std.bytes.to_str_utf8 :: read_back :: call) :: call\n",
            "    std.io.print[Int] :: size :: call\n",
            "    std.io.print[Bool] :: (modified > 0) :: call\n",
            "    if not (unwrap_unit :: (std.fs.remove_file :: source :: call) :: call):\n",
            "        return 8\n",
            "    if not (unwrap_unit :: (std.fs.remove_file :: moved :: call) :: call):\n",
            "        return 9\n",
            "    if not (unwrap_unit :: (std.fs.remove_dir_all :: data_dir :: call) :: call):\n",
            "        return 10\n",
            "    std.io.print[Bool] :: (std.fs.exists :: data_dir :: call) :: call\n",
            "    return 0\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let graph = load_workspace_graph(&dir).expect("workspace graph should load");
    let checked = check_workspace_graph(&graph).expect("workspace should check");
    let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
        .expect("fingerprints should compute");
    let order = plan_workspace(&graph).expect("workspace order should plan");
    let statuses =
        plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
    execute_build(&graph, &statuses).expect("build should execute");

    let artifact_path = graph.root_dir.join(
        &statuses
            .iter()
            .find(|status| status.member == "runtime_std_fs_bytes")
            .expect("app artifact status should exist")
            .artifact_rel_path,
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let cwd = dir.join("fixture").to_string_lossy().replace('\\', "/");
    fs::create_dir_all(dir.join("fixture")).expect("fixture root should exist");
    let mut host = BufferedHost {
        cwd: cwd.clone(),
        sandbox_root: cwd,
        ..BufferedHost::default()
    };
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(
        host.stdout,
        vec![
            "true".to_string(),
            "arc".to_string(),
            "3".to_string(),
            "true".to_string(),
            "false".to_string(),
        ]
    );

    let fixture_root = dir.join("fixture");
    assert!(!fixture_root.join("data").exists());
    assert!(!fixture_root.join("moved.bin").exists());
    assert!(!fixture_root.join("empty").exists());

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_linked_std_fs_stream_routines() {
    let dir = temp_workspace_dir("std_fs_streams");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_std_fs_streams\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.bytes\n",
            "import std.fs\n",
            "use std.result.Result\n",
            "fn write_and_close(take stream: std.fs.FileStream, read bytes: Array[Int]) -> Int:\n",
            "    let mut stream = stream\n",
            "    let wrote = match (std.fs.stream_write :: stream, bytes :: call):\n",
            "        Result.Ok(count) => count\n",
            "        Result.Err(_) => -1\n",
            "    if wrote < 0:\n",
            "        return 1\n",
            "    if wrote != (std.bytes.len :: bytes :: call):\n",
            "        return 2\n",
            "    let close_result = std.fs.stream_close :: stream :: call\n",
            "    if close_result :: :: is_err:\n",
            "        return 3\n",
            "    return 0\n",
            "fn verify_read(take stream: std.fs.FileStream) -> Int:\n",
            "    let mut stream = stream\n",
            "    let empty = std.bytes.from_str_utf8 :: \"\" :: call\n",
            "    let first_result = std.fs.stream_read :: stream, 5 :: call\n",
            "    if first_result :: :: is_err:\n",
            "        return 4\n",
            "    let first = match first_result:\n",
            "        Result.Ok(bytes) => bytes\n",
            "        Result.Err(_) => empty\n",
            "    if (std.bytes.to_str_utf8 :: first :: call) != \"hello\":\n",
            "        return 5\n",
            "    let before_eof_result = std.fs.stream_eof :: stream :: call\n",
            "    if before_eof_result :: :: is_err:\n",
            "        return 6\n",
            "    let before_eof = match before_eof_result:\n",
            "        Result.Ok(value) => value\n",
            "        Result.Err(_) => false\n",
            "    if before_eof:\n",
            "        return 7\n",
            "    let second_result = std.fs.stream_read :: stream, 5 :: call\n",
            "    if second_result :: :: is_err:\n",
            "        return 8\n",
            "    let second = match second_result:\n",
            "        Result.Ok(bytes) => bytes\n",
            "        Result.Err(_) => empty\n",
            "    if (std.bytes.to_str_utf8 :: second :: call) != \"!\":\n",
            "        return 9\n",
            "    let after_eof_result = std.fs.stream_eof :: stream :: call\n",
            "    if after_eof_result :: :: is_err:\n",
            "        return 10\n",
            "    let after_eof = match after_eof_result:\n",
            "        Result.Ok(value) => value\n",
            "        Result.Err(_) => false\n",
            "    if not after_eof:\n",
            "        return 11\n",
            "    let close_result = std.fs.stream_close :: stream :: call\n",
            "    if close_result :: :: is_err:\n",
            "        return 12\n",
            "    return 0\n",
            "fn main() -> Int:\n",
            "    let hello = std.bytes.from_str_utf8 :: \"hello\" :: call\n",
            "    let bang = std.bytes.from_str_utf8 :: \"!\" :: call\n",
            "    let write_status = match (std.fs.stream_open_write :: \"notes.bin\", false :: call):\n",
            "        Result.Ok(stream) => write_and_close :: stream, hello :: call\n",
            "        Result.Err(_) => 20\n",
            "    if write_status != 0:\n",
            "        return 21\n",
            "    let append_status = match (std.fs.stream_open_write :: \"notes.bin\", true :: call):\n",
            "        Result.Ok(stream) => write_and_close :: stream, bang :: call\n",
            "        Result.Err(_) => 22\n",
            "    if append_status != 0:\n",
            "        return 23\n",
            "    let read_status = match (std.fs.stream_open_read :: \"notes.bin\" :: call):\n",
            "        Result.Ok(stream) => verify_read :: stream :: call\n",
            "        Result.Err(_) => 24\n",
            "    if read_status != 0:\n",
            "        return 25\n",
            "    return 0\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let graph = load_workspace_graph(&dir).expect("workspace graph should load");
    let checked = check_workspace_graph(&graph).expect("workspace should check");
    let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
        .expect("fingerprints should compute");
    let order = plan_workspace(&graph).expect("workspace order should plan");
    let statuses =
        plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
    execute_build(&graph, &statuses).expect("build should execute");

    let artifact_path = graph.root_dir.join(
        &statuses
            .iter()
            .find(|status| status.member == "runtime_std_fs_streams")
            .expect("app artifact status should exist")
            .artifact_rel_path,
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let fixture_root = dir.join("fixture");
    fs::create_dir_all(&fixture_root).expect("fixture root should exist");
    let cwd = fixture_root.to_string_lossy().replace('\\', "/");
    let mut host = BufferedHost {
        cwd: cwd.clone(),
        sandbox_root: cwd,
        ..BufferedHost::default()
    };
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(
        fs::read_to_string(fixture_root.join("notes.bin")).expect("streamed file should exist"),
        "hello!"
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_local_record_constructor_and_impl_method() {
    let dir = temp_workspace_dir("record_method");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_record_method\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.io\n",
            "record Counter:\n",
            "    value: Int\n",
            "impl Counter:\n",
            "    fn double(read self: Counter) -> Int:\n",
            "        return self.value * 2\n",
            "fn main() -> Int:\n",
            "    let counter = Counter :: value = 7 :: call\n",
            "    std.io.print[Int] :: counter.value :: call\n",
            "    std.io.print[Int] :: (counter :: :: double) :: call\n",
            "    return 0\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let graph = load_workspace_graph(&dir).expect("workspace graph should load");
    let checked = check_workspace_graph(&graph).expect("workspace should check");
    let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
        .expect("fingerprints should compute");
    let order = plan_workspace(&graph).expect("workspace order should plan");
    let statuses =
        plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
    execute_build(&graph, &statuses).expect("build should execute");

    let artifact_path = graph.root_dir.join(
        &statuses
            .iter()
            .find(|status| status.member == "runtime_record_method")
            .expect("app artifact status should exist")
            .artifact_rel_path,
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(host.stdout, vec!["7".to_string(), "14".to_string()]);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_linked_std_process_routines() {
    let dir = temp_workspace_dir("std_process");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_std_process\"\nkind = \"app\"\n",
    );
    let (program, status_a, status_b, capture_a, capture_b) = if cfg!(windows) {
        ("cmd", "/C", "exit 7", "/C", "echo hello")
    } else {
        ("sh", "-c", "exit 7", "-c", "printf hello")
    };
    write_file(
        &dir.join("src").join("shelf.arc"),
        &format!(
            concat!(
                "import std.bytes\n",
                "import std.collections.list\n",
                "import std.io\n",
                "import std.process\n",
                "import std.text\n",
                "use std.result.Result\n",
                "fn status_args() -> List[Str]:\n",
                "    let mut args = std.collections.list.new[Str] :: :: call\n",
                "    args :: {status_a:?} :: push\n",
                "    args :: {status_b:?} :: push\n",
                "    return args\n",
                "fn capture_args() -> List[Str]:\n",
                "    let mut args = std.collections.list.new[Str] :: :: call\n",
                "    args :: {capture_a:?} :: push\n",
                "    args :: {capture_b:?} :: push\n",
                "    return args\n",
                "fn main() -> Int:\n",
                "    let status = match (std.process.exec_status :: {program:?}, (status_args :: :: call) :: call):\n",
                "        Result.Ok(value) => value\n",
                "        Result.Err(_) => -1\n",
                "    let capture_result = std.process.exec_capture :: {program:?}, (capture_args :: :: call) :: call\n",
                "    if capture_result :: :: is_err:\n",
                "        return 99\n",
                "    let empty = std.bytes.from_str_utf8 :: \"\" :: call\n",
                "    let capture = capture_result :: (std.process.ExecCapture :: status = 0, output = (empty, empty), utf8 = (true, true) :: call) :: unwrap_or\n",
                "    let text = match (capture :: :: stdout_text):\n",
                "        Result.Ok(value) => value\n",
                "        Result.Err(_) => \"\"\n",
                "    std.io.print[Int] :: status :: call\n",
                "    std.io.print[Bool] :: (capture :: :: success) :: call\n",
                "    std.io.print[Bool] :: (std.text.starts_with :: text, \"hello\" :: call) :: call\n",
                "    return 0\n",
            ),
            program = program,
            status_a = status_a,
            status_b = status_b,
            capture_a = capture_a,
            capture_b = capture_b,
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let graph = load_workspace_graph(&dir).expect("workspace graph should load");
    let checked = check_workspace_graph(&graph).expect("workspace should check");
    let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
        .expect("fingerprints should compute");
    let order = plan_workspace(&graph).expect("workspace order should plan");
    let statuses =
        plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
    execute_build(&graph, &statuses).expect("build should execute");

    let artifact_path = graph.root_dir.join(
        &statuses
            .iter()
            .find(|status| status.member == "runtime_std_process")
            .expect("app artifact status should exist")
            .artifact_rel_path,
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost {
        allow_process: true,
        ..BufferedHost::default()
    };
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(
        host.stdout,
        vec!["7".to_string(), "true".to_string(), "true".to_string()]
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_linked_std_option_routines() {
    let dir = temp_workspace_dir("std_option");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_std_option\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.option\n",
            "import std.io\n",
            "use std.option.Option\n",
            "fn main() -> Int:\n",
            "    let some = Option.Some[Int] :: 5 :: call\n",
            "    let none = Option.None[Int] :: :: call\n",
            "    std.io.print[Bool] :: (some :: :: is_some) :: call\n",
            "    std.io.print[Bool] :: (none :: :: is_none) :: call\n",
            "    std.io.print[Int] :: (some :: 0 :: unwrap_or) :: call\n",
            "    std.io.print[Int] :: (none :: 9 :: unwrap_or) :: call\n",
            "    return 0\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let graph = load_workspace_graph(&dir).expect("workspace graph should load");
    let checked = check_workspace_graph(&graph).expect("workspace should check");
    let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
        .expect("fingerprints should compute");
    let order = plan_workspace(&graph).expect("workspace order should plan");
    let statuses =
        plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
    execute_build(&graph, &statuses).expect("build should execute");

    let artifact_path = graph.root_dir.join(
        &statuses
            .iter()
            .find(|status| status.member == "runtime_std_option")
            .expect("app artifact status should exist")
            .artifact_rel_path,
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(
        host.stdout,
        vec![
            "true".to_string(),
            "true".to_string(),
            "5".to_string(),
            "9".to_string(),
        ]
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_named_qualifier_path_routines() {
    let dir = temp_workspace_dir("named_qualifier_path");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_named_qualifier_path\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.io\n",
            "import std.text\n",
            "use std.text as texts\n",
            "fn main() -> Int:\n",
            "    std.io.print[Bool] :: (\"arcana\" :: \"arc\" :: texts.starts_with) :: call\n",
            "    std.io.print[Bool] :: (\"arcana\" :: \"ana\" :: texts.ends_with) :: call\n",
            "    std.io.print[Int] :: (\"arcana\" :: 0, \"can\" :: texts.find) :: call\n",
            "    return 0\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let graph = load_workspace_graph(&dir).expect("workspace graph should load");
    let checked = check_workspace_graph(&graph).expect("workspace should check");
    let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
        .expect("fingerprints should compute");
    let order = plan_workspace(&graph).expect("workspace order should plan");
    let statuses =
        plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
    execute_build(&graph, &statuses).expect("build should execute");

    let artifact_path = graph.root_dir.join(
        &statuses
            .iter()
            .find(|status| status.member == "runtime_named_qualifier_path")
            .expect("app artifact status should exist")
            .artifact_rel_path,
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(
        host.stdout,
        vec!["true".to_string(), "true".to_string(), "2".to_string()]
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_linked_std_result_routines() {
    let dir = temp_workspace_dir("std_result");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_std_result\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.io\n",
            "import std.result\n",
            "use std.result.Result\n",
            "fn main() -> Int:\n",
            "    let ok = Result.Ok[Int, Str] :: 7 :: call\n",
            "    let err = Result.Err[Int, Str] :: \"bad\" :: call\n",
            "    std.io.print[Bool] :: (ok :: :: is_ok) :: call\n",
            "    std.io.print[Bool] :: (err :: :: is_err) :: call\n",
            "    std.io.print[Int] :: (ok :: 0 :: unwrap_or) :: call\n",
            "    std.io.print[Int] :: (err :: 13 :: unwrap_or) :: call\n",
            "    return 0\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let graph = load_workspace_graph(&dir).expect("workspace graph should load");
    let checked = check_workspace_graph(&graph).expect("workspace should check");
    let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
        .expect("fingerprints should compute");
    let order = plan_workspace(&graph).expect("workspace order should plan");
    let statuses =
        plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
    execute_build(&graph, &statuses).expect("build should execute");

    let artifact_path = graph.root_dir.join(
        &statuses
            .iter()
            .find(|status| status.member == "runtime_std_result")
            .expect("app artifact status should exist")
            .artifact_rel_path,
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(
        host.stdout,
        vec![
            "true".to_string(),
            "true".to_string(),
            "7".to_string(),
            "13".to_string(),
        ]
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_try_qualifier_routines() {
    let dir = temp_workspace_dir("try_qualifier");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_try_qualifier\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.io\n",
            "import std.result\n",
            "use std.result.Result\n",
            "fn parse(flag: Bool) -> Result[Int, Str]:\n",
            "    if flag:\n",
            "        return Result.Ok[Int, Str] :: 7 :: call\n",
            "    return Result.Err[Int, Str] :: \"bad\" :: call\n",
            "fn add_one(flag: Bool) -> Result[Int, Str]:\n",
            "    let value = (parse :: flag :: call) :: :: ?\n",
            "    return Result.Ok[Int, Str] :: value + 1 :: call\n",
            "fn main() -> Int:\n",
            "    let ok = add_one :: true :: call\n",
            "    let err = add_one :: false :: call\n",
            "    let ok_value = match ok:\n",
            "        Result.Ok(value) => value\n",
            "        Result.Err(_) => -1\n",
            "    let err_value = match err:\n",
            "        Result.Ok(_) => \"\"\n",
            "        Result.Err(message) => message\n",
            "    std.io.print[Int] :: ok_value :: call\n",
            "    std.io.print[Str] :: err_value :: call\n",
            "    return 0\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let graph = load_workspace_graph(&dir).expect("workspace graph should load");
    let checked = check_workspace_graph(&graph).expect("workspace should check");
    let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
        .expect("fingerprints should compute");
    let order = plan_workspace(&graph).expect("workspace order should plan");
    let statuses =
        plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
    execute_build(&graph, &statuses).expect("build should execute");

    let artifact_path = graph.root_dir.join(
        &statuses
            .iter()
            .find(|status| status.member == "runtime_try_qualifier")
            .expect("app artifact status should exist")
            .artifact_rel_path,
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(host.stdout, vec!["8".to_string(), "bad".to_string()]);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_rejects_try_qualifier_arguments() {
    let plan = RuntimePackagePlan {
        package_name: "try_args_runtime".to_string(),
        root_module_id: "try_args_runtime".to_string(),
        direct_deps: Vec::new(),
        runtime_requirements: Vec::new(),
        module_aliases: BTreeMap::new(),
        entrypoints: vec![RuntimeEntrypointPlan {
            module_id: "try_args_runtime".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            is_async: false,
            exported: true,
            routine_index: 0,
        }],
        routines: vec![RuntimeRoutinePlan {
            module_id: "try_args_runtime".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            exported: true,
            is_async: false,
            type_params: Vec::new(),
            behavior_attrs: BTreeMap::new(),
            params: Vec::new(),
            signature_row: "fn main() -> Int:".to_string(),
            intrinsic_impl: None,
            foreword_rows: Vec::new(),
            rollup_rows: Vec::new(),
            rollups: Vec::new(),
            statements: vec![
                ParsedStmt::Let {
                    mutable: false,
                    name: "value".to_string(),
                    value: ParsedExpr::Phrase {
                        subject: Box::new(ParsedExpr::Path(vec![
                            "Result".to_string(),
                            "Ok".to_string(),
                        ])),
                        args: vec![ParsedPhraseArg {
                            name: None,
                            value: ParsedExpr::Int(1),
                        }],
                        qualifier_kind: ParsedPhraseQualifierKind::Call,
                        qualifier: "call".to_string(),
                        attached: Vec::new(),
                    },
                },
                ParsedStmt::Expr {
                    expr: ParsedExpr::Phrase {
                        subject: Box::new(ParsedExpr::Path(vec!["value".to_string()])),
                        args: vec![ParsedPhraseArg {
                            name: None,
                            value: ParsedExpr::Int(0),
                        }],
                        qualifier_kind: ParsedPhraseQualifierKind::Try,
                        qualifier: "?".to_string(),
                        attached: Vec::new(),
                    },
                    rollups: Vec::new(),
                },
                ParsedStmt::Return(Some(ParsedExpr::Int(0))),
            ],
        }],
    };
    let mut host = BufferedHost::default();
    let err =
        execute_main(&plan, &mut host).expect_err("runtime should reject try qualifier arguments");

    assert!(err.contains("`:: ?` does not accept arguments"), "{err}");
}

#[test]
fn execute_main_runs_linked_std_collection_method_routines() {
    let dir = temp_workspace_dir("std_collection_methods");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_std_collection_methods\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.collections.array\n",
            "import std.collections.list\n",
            "import std.collections.map\n",
            "import std.io\n",
            "fn main() -> Int:\n",
            "    let mut xs = std.collections.list.new[Int] :: :: call\n",
            "    xs :: 4 :: push\n",
            "    xs :: 7 :: push\n",
            "    std.io.print[Int] :: (xs :: :: len) :: call\n",
            "    std.io.print[Int] :: (xs :: :: pop) :: call\n",
            "    let popped = xs :: 9 :: try_pop_or\n",
            "    std.io.print[Bool] :: popped.0 :: call\n",
            "    std.io.print[Int] :: popped.1 :: call\n",
            "    let fallback = xs :: 11 :: try_pop_or\n",
            "    std.io.print[Bool] :: fallback.0 :: call\n",
            "    std.io.print[Int] :: fallback.1 :: call\n",
            "    let arr = std.collections.array.new[Int] :: 2, 5 :: call\n",
            "    std.io.print[Int] :: ((arr :: :: to_list) :: :: len) :: call\n",
            "    let mut mapping = std.collections.map.new[Str, Int] :: :: call\n",
            "    mapping :: \"a\", 1 :: set\n",
            "    std.io.print[Int] :: (mapping :: :: len) :: call\n",
            "    return 0\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let graph = load_workspace_graph(&dir).expect("workspace graph should load");
    let checked = check_workspace_graph(&graph).expect("workspace should check");
    let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
        .expect("fingerprints should compute");
    let order = plan_workspace(&graph).expect("workspace order should plan");
    let statuses =
        plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
    execute_build(&graph, &statuses).expect("build should execute");

    let artifact_path = graph.root_dir.join(
        &statuses
            .iter()
            .find(|status| status.member == "runtime_std_collection_methods")
            .expect("app artifact status should exist")
            .artifact_rel_path,
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(
        host.stdout,
        vec![
            "2".to_string(),
            "7".to_string(),
            "true".to_string(),
            "4".to_string(),
            "false".to_string(),
            "11".to_string(),
            "2".to_string(),
            "1".to_string(),
        ]
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_range_index_slice_and_literal_match_routines() {
    let dir = temp_workspace_dir("range_index_slice_match");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_range_index_slice_match\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.collections.array\n",
            "import std.io\n",
            "fn main() -> Int:\n",
            "    let xs = [10, 20, 30, 40]\n",
            "    std.io.print[Int] :: xs[0] :: call\n",
            "    let tail = xs[1..]\n",
            "    std.io.print[Int] :: (tail :: :: len) :: call\n",
            "    std.io.print[Int] :: tail[0] :: call\n",
            "    let mid = xs[1..=2]\n",
            "    std.io.print[Int] :: (mid :: :: len) :: call\n",
            "    std.io.print[Int] :: mid[1] :: call\n",
            "    let whole = xs[..]\n",
            "    std.io.print[Int] :: (whole :: :: len) :: call\n",
            "    let arr = std.collections.array.new[Int] :: 3, 5 :: call\n",
            "    std.io.print[Int] :: arr[1] :: call\n",
            "    std.io.print[Int] :: ((arr[1..]) :: :: len) :: call\n",
            "    let mut sum = 0\n",
            "    for i in 1..4:\n",
            "        sum = sum + i\n",
            "    std.io.print[Int] :: sum :: call\n",
            "    let r1 = 1..4\n",
            "    let r2 = 1..4\n",
            "    let r3 = ..=3\n",
            "    let r4 = ..=3\n",
            "    std.io.print[Bool] :: (r1 == r2) :: call\n",
            "    std.io.print[Bool] :: (r3 == r4) :: call\n",
            "    let as_text = match 2:\n",
            "        1 => \"one\"\n",
            "        2 => \"two\"\n",
            "        _ => \"other\"\n",
            "    std.io.print[Str] :: as_text :: call\n",
            "    let flag = match false:\n",
            "        true => \"yes\"\n",
            "        false => \"no\"\n",
            "    std.io.print[Str] :: flag :: call\n",
            "    let fruit = match \"pear\":\n",
            "        \"apple\" => \"miss\"\n",
            "        \"pear\" => \"hit\"\n",
            "        _ => \"other\"\n",
            "    std.io.print[Str] :: fruit :: call\n",
            "    return 0\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let graph = load_workspace_graph(&dir).expect("workspace graph should load");
    let checked = check_workspace_graph(&graph).expect("workspace should check");
    let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
        .expect("fingerprints should compute");
    let order = plan_workspace(&graph).expect("workspace order should plan");
    let statuses =
        plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
    execute_build(&graph, &statuses).expect("build should execute");

    let artifact_path = graph.root_dir.join(
        &statuses
            .iter()
            .find(|status| status.member == "runtime_range_index_slice_match")
            .expect("app artifact status should exist")
            .artifact_rel_path,
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(
        host.stdout,
        vec![
            "10".to_string(),
            "3".to_string(),
            "20".to_string(),
            "2".to_string(),
            "30".to_string(),
            "4".to_string(),
            "5".to_string(),
            "2".to_string(),
            "6".to_string(),
            "true".to_string(),
            "true".to_string(),
            "two".to_string(),
            "no".to_string(),
            "hit".to_string(),
        ]
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_indexed_assignment_routines() {
    let dir = temp_workspace_dir("indexed_assignment");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_indexed_assignment\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.collections.array\n",
            "import std.io\n",
            "fn main() -> Int:\n",
            "    let mut xs = [1, 2, 3]\n",
            "    xs[1] = 9\n",
            "    xs[2] += 5\n",
            "    std.io.print[Int] :: xs[1] :: call\n",
            "    std.io.print[Int] :: xs[2] :: call\n",
            "    let mut arr = std.collections.array.new[Int] :: 3, 4 :: call\n",
            "    arr[0] = 7\n",
            "    arr[2] += 3\n",
            "    std.io.print[Int] :: arr[0] :: call\n",
            "    std.io.print[Int] :: arr[2] :: call\n",
            "    return 0\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let graph = load_workspace_graph(&dir).expect("workspace graph should load");
    let checked = check_workspace_graph(&graph).expect("workspace should check");
    let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
        .expect("fingerprints should compute");
    let order = plan_workspace(&graph).expect("workspace order should plan");
    let statuses =
        plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
    execute_build(&graph, &statuses).expect("build should execute");

    let artifact_path = graph.root_dir.join(
        &statuses
            .iter()
            .find(|status| status.member == "runtime_indexed_assignment")
            .expect("app artifact status should exist")
            .artifact_rel_path,
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(
        host.stdout,
        vec![
            "9".to_string(),
            "8".to_string(),
            "7".to_string(),
            "7".to_string(),
        ]
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_rejects_use_after_take_move() {
    let plan = RuntimePackagePlan {
        package_name: "take_move_runtime".to_string(),
        root_module_id: "take_move_runtime".to_string(),
        direct_deps: Vec::new(),
        runtime_requirements: Vec::new(),
        module_aliases: BTreeMap::new(),
        entrypoints: vec![RuntimeEntrypointPlan {
            module_id: "take_move_runtime".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            is_async: false,
            exported: true,
            routine_index: 2,
        }],
        routines: vec![
            RuntimeRoutinePlan {
                module_id: "take_move_runtime".to_string(),
                symbol_name: "consume".to_string(),
                symbol_kind: "fn".to_string(),
                exported: false,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: vec![RuntimeParamPlan {
                    mode: Some("take".to_string()),
                    name: "value".to_string(),
                    ty: "Str".to_string(),
                }],
                signature_row: "fn consume(take value: Str) -> Int:".to_string(),
                intrinsic_impl: None,
                foreword_rows: Vec::new(),
                rollup_rows: Vec::new(),
                rollups: Vec::new(),
                statements: vec![ParsedStmt::Return(Some(ParsedExpr::Int(1)))],
            },
            RuntimeRoutinePlan {
                module_id: "take_move_runtime".to_string(),
                symbol_name: "reuse".to_string(),
                symbol_kind: "fn".to_string(),
                exported: false,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: vec![RuntimeParamPlan {
                    mode: Some("read".to_string()),
                    name: "value".to_string(),
                    ty: "Str".to_string(),
                }],
                signature_row: "fn reuse(read value: Str) -> Int:".to_string(),
                intrinsic_impl: None,
                foreword_rows: Vec::new(),
                rollup_rows: Vec::new(),
                rollups: Vec::new(),
                statements: vec![ParsedStmt::Return(Some(ParsedExpr::Int(0)))],
            },
            RuntimeRoutinePlan {
                module_id: "take_move_runtime".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: Vec::new(),
                signature_row: "fn main() -> Int:".to_string(),
                intrinsic_impl: None,
                foreword_rows: Vec::new(),
                rollup_rows: Vec::new(),
                rollups: Vec::new(),
                statements: vec![
                    ParsedStmt::Let {
                        mutable: true,
                        name: "s".to_string(),
                        value: ParsedExpr::Str("hi".to_string()),
                    },
                    ParsedStmt::Expr {
                        expr: ParsedExpr::Phrase {
                            subject: Box::new(ParsedExpr::Path(vec!["consume".to_string()])),
                            args: vec![ParsedPhraseArg {
                                name: None,
                                value: ParsedExpr::Path(vec!["s".to_string()]),
                            }],
                            qualifier_kind: ParsedPhraseQualifierKind::Call,
                            qualifier: "call".to_string(),
                            attached: Vec::new(),
                        },
                        rollups: Vec::new(),
                    },
                    ParsedStmt::Return(Some(ParsedExpr::Phrase {
                        subject: Box::new(ParsedExpr::Path(vec!["reuse".to_string()])),
                        args: vec![ParsedPhraseArg {
                            name: None,
                            value: ParsedExpr::Path(vec!["s".to_string()]),
                        }],
                        qualifier_kind: ParsedPhraseQualifierKind::Call,
                        qualifier: "call".to_string(),
                        attached: Vec::new(),
                    })),
                ],
            },
        ],
    };
    let mut host = BufferedHost::default();
    let err = execute_main(&plan, &mut host).expect_err("runtime should reject moved-local use");

    assert!(err.contains("use of moved local `s`"), "{err}");
}

#[test]
fn execute_main_rejects_direct_intrinsic_take_fallback_reuse() {
    let plan = RuntimePackagePlan {
        package_name: "take_intrinsic_runtime".to_string(),
        root_module_id: "take_intrinsic_runtime".to_string(),
        direct_deps: Vec::new(),
        runtime_requirements: Vec::new(),
        module_aliases: BTreeMap::new(),
        entrypoints: vec![RuntimeEntrypointPlan {
            module_id: "take_intrinsic_runtime".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            is_async: false,
            exported: true,
            routine_index: 0,
        }],
        routines: vec![RuntimeRoutinePlan {
            module_id: "take_intrinsic_runtime".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            exported: true,
            is_async: false,
            type_params: Vec::new(),
            behavior_attrs: BTreeMap::new(),
            params: Vec::new(),
            signature_row: "fn main() -> Int:".to_string(),
            intrinsic_impl: None,
            foreword_rows: Vec::new(),
            rollup_rows: Vec::new(),
            rollups: Vec::new(),
            statements: vec![
                ParsedStmt::Let {
                    mutable: true,
                    name: "xs".to_string(),
                    value: ParsedExpr::Collection {
                        items: vec![ParsedExpr::Str("a".to_string())],
                    },
                },
                ParsedStmt::Expr {
                    expr: ParsedExpr::Phrase {
                        subject: Box::new(ParsedExpr::Path(vec![
                            "std".to_string(),
                            "kernel".to_string(),
                            "collections".to_string(),
                            "array_from_list".to_string(),
                        ])),
                        args: vec![ParsedPhraseArg {
                            name: None,
                            value: ParsedExpr::Path(vec!["xs".to_string()]),
                        }],
                        qualifier_kind: ParsedPhraseQualifierKind::Call,
                        qualifier: "call".to_string(),
                        attached: Vec::new(),
                    },
                    rollups: Vec::new(),
                },
                ParsedStmt::Return(Some(ParsedExpr::Phrase {
                    subject: Box::new(ParsedExpr::Path(vec![
                        "std".to_string(),
                        "kernel".to_string(),
                        "collections".to_string(),
                        "list_len".to_string(),
                    ])),
                    args: vec![ParsedPhraseArg {
                        name: None,
                        value: ParsedExpr::Path(vec!["xs".to_string()]),
                    }],
                    qualifier_kind: ParsedPhraseQualifierKind::Call,
                    qualifier: "call".to_string(),
                    attached: Vec::new(),
                })),
            ],
        }],
    };
    let mut host = BufferedHost::default();
    let err = execute_main(&plan, &mut host).expect_err("runtime should reject moved-local reuse");

    assert!(err.contains("use of moved local `xs`"), "{err}");
}

#[test]
fn execute_main_allows_copy_take_and_reassign_after_take_move() {
    let dir = temp_workspace_dir("take_copy_and_reassign");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_take_copy_and_reassign\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.io\n",
            "fn consume_text(take value: Str):\n",
            "    return\n",
            "fn consume_int(take value: Int) -> Int:\n",
            "    return value + 1\n",
            "fn main() -> Int:\n",
            "    let mut s = \"hi\"\n",
            "    consume_text :: s :: call\n",
            "    s = \"bye\"\n",
            "    std.io.print[Str] :: s :: call\n",
            "    let x = 4\n",
            "    std.io.print[Int] :: (consume_int :: x :: call) :: call\n",
            "    std.io.print[Int] :: x :: call\n",
            "    return 0\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let graph = load_workspace_graph(&dir).expect("workspace graph should load");
    let checked = check_workspace_graph(&graph).expect("workspace should check");
    let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
        .expect("fingerprints should compute");
    let order = plan_workspace(&graph).expect("workspace order should plan");
    let statuses =
        plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
    execute_build(&graph, &statuses).expect("build should execute");

    let artifact_path = graph.root_dir.join(
        &statuses
            .iter()
            .find(|status| status.member == "runtime_take_copy_and_reassign")
            .expect("app artifact status should exist")
            .artifact_rel_path,
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(
        host.stdout,
        vec!["bye".to_string(), "5".to_string(), "4".to_string()]
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_apply_and_await_apply_qualifiers() {
    let dir = temp_workspace_dir("apply_and_await_apply");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_apply_and_await_apply\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.io\n",
            "fn add(left: Int, right: Int) -> Int:\n",
            "    return left + right\n",
            "async fn compute(value: Int) -> Int:\n",
            "    return value + 2\n",
            "fn main() -> Int:\n",
            "    std.io.print[Int] :: (add :: 2, 3 :: >) :: call\n",
            "    let task = weave 7\n",
            "    std.io.print[Int] :: (task :: :: >>) :: call\n",
            "    std.io.print[Int] :: (compute :: 5 :: >>) :: call\n",
            "    return 0\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let graph = load_workspace_graph(&dir).expect("workspace graph should load");
    let checked = check_workspace_graph(&graph).expect("workspace should check");
    let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
        .expect("fingerprints should compute");
    let order = plan_workspace(&graph).expect("workspace order should plan");
    let statuses =
        plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
    execute_build(&graph, &statuses).expect("build should execute");

    let artifact_path = graph.root_dir.join(
        &statuses
            .iter()
            .find(|status| status.member == "runtime_apply_and_await_apply")
            .expect("app artifact status should exist")
            .artifact_rel_path,
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(
        host.stdout,
        vec!["5".to_string(), "7".to_string(), "7".to_string()]
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_linked_std_ecs_behavior_routines() {
    let dir = temp_workspace_dir("std_ecs");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_std_ecs\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.behaviors\n",
            "import std.ecs\n",
            "import std.io\n",
            "record Position:\n",
            "    x: Int\n",
            "    y: Int\n",
            "behavior[phase=startup] fn boot() -> Int:\n",
            "    std.ecs.set_component[Int] :: 7 :: call\n",
            "    let entity = std.ecs.spawn :: :: call\n",
            "    std.ecs.set_component_at[Position] :: entity, (Position :: x = 4, y = 5 :: call) :: call\n",
            "    return 0\n",
            "behavior[phase=update] fn tick() -> Int:\n",
            "    if not (std.ecs.has_component[Int] :: :: call):\n",
            "        return 10\n",
            "    let current = std.ecs.get_component[Int] :: :: call\n",
            "    std.ecs.set_component[Int] :: current + 1 :: call\n",
            "    return 0\n",
            "system[phase=update] fn cleanup() -> Int:\n",
            "    if not (std.ecs.has_component_at[Position] :: 1 :: call):\n",
            "        return 20\n",
            "    let pos = std.ecs.get_component_at[Position] :: 1 :: call\n",
            "    if pos.x != 4:\n",
            "        return 21\n",
            "    if pos.y != 5:\n",
            "        return 22\n",
            "    let current = std.ecs.get_component[Int] :: :: call\n",
            "    std.ecs.set_component[Int] :: current + 10 :: call\n",
            "    std.ecs.remove_component_at[Position] :: 1 :: call\n",
            "    std.ecs.despawn :: 1 :: call\n",
            "    return 0\n",
            "behavior[phase=render] fn render_only() -> Int:\n",
            "    std.ecs.set_component[Int] :: 999 :: call\n",
            "    return 0\n",
            "fn main() -> Int:\n",
            "    if (std.ecs.step_startup :: :: call) != 0:\n",
            "        return 1\n",
            "    if (std.behaviors.step :: \"update\" :: call) != 0:\n",
            "        return 2\n",
            "    if (std.ecs.get_component[Int] :: :: call) != 18:\n",
            "        return 3\n",
            "    if std.ecs.has_component_at[Position] :: 1 :: call:\n",
            "        return 4\n",
            "    std.io.print[Int] :: (std.ecs.get_component[Int] :: :: call) :: call\n",
            "    return 0\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let graph = load_workspace_graph(&dir).expect("workspace graph should load");
    let checked = check_workspace_graph(&graph).expect("workspace should check");
    let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
        .expect("fingerprints should compute");
    let order = plan_workspace(&graph).expect("workspace order should plan");
    let statuses =
        plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
    execute_build(&graph, &statuses).expect("build should execute");

    let artifact_path = graph.root_dir.join(
        &statuses
            .iter()
            .find(|status| status.member == "runtime_std_ecs")
            .expect("app artifact status should exist")
            .artifact_rel_path,
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(host.stdout, vec!["18".to_string()]);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_owned_app_facade_workspace() {
    let dir = temp_workspace_dir("owned_app_facade");
    let desktop_dep = repo_root()
        .join("grimoires")
        .join("owned")
        .join("app")
        .join("arcana-desktop")
        .to_string_lossy()
        .replace('\\', "/");
    let audio_dep = repo_root()
        .join("grimoires")
        .join("owned")
        .join("app")
        .join("arcana-audio")
        .to_string_lossy()
        .replace('\\', "/");
    write_file(
        &dir.join("book.toml"),
        &format!(
            concat!(
                "name = \"runtime_owned_app_facade\"\n",
                "kind = \"app\"\n",
                "[deps]\n",
                "arcana_desktop = {desktop_dep:?}\n",
                "arcana_audio = {audio_dep:?}\n",
            ),
            desktop_dep = desktop_dep,
            audio_dep = audio_dep,
        ),
    );
    write_file(&dir.join("fixture").join("clip.wav"), "wave");
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import arcana_audio.clip\n",
            "import arcana_audio.output\n",
            "import arcana_audio.playback\n",
            "import arcana_desktop.events\n",
            "import arcana_desktop.input\n",
            "import arcana_desktop.window\n",
            "import std.io\n",
            "use std.result.Result\n",
            "fn with_playback(take win: std.window.Window, take device: std.audio.AudioDevice, take playback: std.audio.AudioPlayback) -> Int:\n",
            "    std.io.print[Bool] :: (arcana_audio.playback.playing :: playback :: call) :: call\n",
            "    let stop = arcana_audio.playback.stop :: playback :: call\n",
            "    if stop :: :: is_err:\n",
            "        return 7\n",
            "    let close_audio = arcana_audio.output.close :: device :: call\n",
            "    if close_audio :: :: is_err:\n",
            "        return 8\n",
            "    let close_window = arcana_desktop.window.close :: win :: call\n",
            "    if close_window :: :: is_err:\n",
            "        return 9\n",
            "    return 0\n",
            "fn with_clip(take win: std.window.Window, take device: std.audio.AudioDevice, read clip: std.audio.AudioBuffer) -> Int:\n",
            "    let mut device = device\n",
            "    let info = arcana_audio.clip.info :: clip :: call\n",
            "    if info.sample_rate_hz != 48000:\n",
            "        return 5\n",
            "    let playback_result = arcana_audio.playback.play :: device, clip :: call\n",
            "    return match playback_result:\n",
            "        Result.Ok(value) => with_playback :: win, device, value :: call\n",
            "        Result.Err(_) => 6\n",
            "fn with_device(take win: std.window.Window, take device: std.audio.AudioDevice) -> Int:\n",
            "    let mut device = device\n",
            "    let cfg = arcana_audio.output.default_output_config :: :: call\n",
            "    arcana_audio.output.configure :: device, cfg :: call\n",
            "    std.io.print[Int] :: (arcana_audio.output.sample_rate_hz :: device :: call) :: call\n",
            "    return match (arcana_audio.clip.load_wav :: \"clip.wav\" :: call):\n",
            "        Result.Ok(value) => with_clip :: win, device, value :: call\n",
            "        Result.Err(_) => 4\n",
            "fn with_window(take win: std.window.Window) -> Int:\n",
            "    let mut win = win\n",
            "    if not (arcana_desktop.window.alive :: win :: call):\n",
            "        return 2\n",
            "    let frame = arcana_desktop.events.pump :: win :: call\n",
            "    let key = arcana_desktop.input.key_code :: \"A\" :: call\n",
            "    std.io.print[Bool] :: (arcana_desktop.input.key_down :: frame, key :: call) :: call\n",
            "    return match (arcana_audio.output.default_output :: :: call):\n",
            "        Result.Ok(value) => with_device :: win, value :: call\n",
            "        Result.Err(_) => 3\n",
            "fn main() -> Int:\n",
            "    return match (arcana_desktop.window.open :: \"Arcana\", 320, 200 :: call):\n",
            "        Result.Ok(value) => with_window :: value :: call\n",
            "        Result.Err(_) => 1\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let graph = load_workspace_graph(&dir).expect("workspace graph should load");
    let checked = check_workspace_graph(&graph).expect("workspace should check");
    let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
        .expect("fingerprints should compute");
    let order = plan_workspace(&graph).expect("workspace order should plan");
    let statuses =
        plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
    execute_build(&graph, &statuses).expect("build should execute");

    let artifact_path = graph.root_dir.join(
        &statuses
            .iter()
            .find(|status| status.member == "runtime_owned_app_facade")
            .expect("app artifact status should exist")
            .artifact_rel_path,
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let fixture_root = dir.join("fixture");
    let mut host = synthetic_window_canvas_host(&fixture_root);
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(
        host.stdout,
        vec!["true".to_string(), "48000".to_string(), "true".to_string()]
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_synthetic_audio_runtime() {
    let dir = temp_workspace_dir("std_audio");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_std_audio\"\nkind = \"app\"\n",
    );
    write_file(&dir.join("fixture").join("clip.wav"), "wave");
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.audio\n",
            "use std.result.Result\n",
            "fn use_playback(take device: std.audio.AudioDevice, take playback: std.audio.AudioPlayback) -> Int:\n",
            "    let mut device = device\n",
            "    let mut playback = playback\n",
            "    if not (playback :: :: playing):\n",
            "        return 9\n",
            "    if playback :: :: paused:\n",
            "        return 10\n",
            "    if playback :: :: finished:\n",
            "        return 11\n",
            "    playback :: :: pause\n",
            "    if not (playback :: :: paused):\n",
            "        return 12\n",
            "    playback :: :: resume\n",
            "    playback :: 500 :: set_gain_milli\n",
            "    playback :: true :: set_looping\n",
            "    if not (playback :: :: looping):\n",
            "        return 13\n",
            "    if (playback :: :: position_frames) != 0:\n",
            "        return 14\n",
            "    let stop = playback :: :: stop\n",
            "    if stop :: :: is_err:\n",
            "        return 15\n",
            "    let close = std.audio.output_close :: device :: call\n",
            "    if close :: :: is_err:\n",
            "        return 16\n",
            "    return 0\n",
            "fn use_clip(take device: std.audio.AudioDevice, read clip: std.audio.AudioBuffer) -> Int:\n",
            "    let mut device = device\n",
            "    if (std.audio.buffer_frames :: clip :: call) != 64:\n",
            "        return 5\n",
            "    if (std.audio.buffer_channels :: clip :: call) != 2:\n",
            "        return 6\n",
            "    if (std.audio.buffer_sample_rate_hz :: clip :: call) != 48000:\n",
            "        return 7\n",
            "    let playback_result = std.audio.play_buffer :: device, clip :: call\n",
            "    return match playback_result:\n",
            "        Result.Ok(value) => use_playback :: device, value :: call\n",
            "        Result.Err(_) => 8\n",
            "fn use_device(take device: std.audio.AudioDevice) -> Int:\n",
            "    let mut device = device\n",
            "    if (std.audio.output_sample_rate_hz :: device :: call) != 48000:\n",
            "        return 2\n",
            "    if (std.audio.output_channels :: device :: call) != 2:\n",
            "        return 3\n",
            "    std.audio.output_set_gain_milli :: device, 750 :: call\n",
            "    return match (std.audio.buffer_load_wav :: \"clip.wav\" :: call):\n",
            "        Result.Ok(value) => use_clip :: device, value :: call\n",
            "        Result.Err(_) => 4\n",
            "fn main() -> Int:\n",
            "    return match (std.audio.default_output :: :: call):\n",
            "        Result.Ok(value) => use_device :: value :: call\n",
            "        Result.Err(_) => 1\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let graph = load_workspace_graph(&dir).expect("workspace graph should load");
    let checked = check_workspace_graph(&graph).expect("workspace should check");
    let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
        .expect("fingerprints should compute");
    let order = plan_workspace(&graph).expect("workspace order should plan");
    let statuses =
        plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
    execute_build(&graph, &statuses).expect("build should execute");

    let artifact_path = graph.root_dir.join(
        &statuses
            .iter()
            .find(|status| status.member == "runtime_std_audio")
            .expect("app artifact status should exist")
            .artifact_rel_path,
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let fixture_root = dir.join("fixture");
    let mut host = synthetic_audio_host(&fixture_root);
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(
        host.audio_log,
        vec![
            "default_output:0".to_string(),
            "output_set_gain_milli:0,750".to_string(),
            format!(
                "buffer_load_wav:{}/clip.wav",
                fixture_root.to_string_lossy().replace('\\', "/")
            ),
            format!(
                "play_buffer:0,0,{}/clip.wav",
                fixture_root.to_string_lossy().replace('\\', "/")
            ),
            "playback_pause:0".to_string(),
            "playback_resume:0".to_string(),
            "playback_set_gain_milli:0,500".to_string(),
            "playback_set_looping:0,true".to_string(),
            "playback_stop:0".to_string(),
            "output_close:0".to_string(),
        ]
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_synthetic_window_canvas_events_runtime() {
    let dir = temp_workspace_dir("std_window_canvas");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_window_canvas\"\nkind = \"app\"\n",
    );
    write_file(&dir.join("fixture").join("sprite.bin"), "sprite");
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.canvas\n",
            "import std.events\n",
            "import std.input\n",
            "import std.time\n",
            "import std.window\n",
            "use std.result.Result\n",
            "fn draw_image(edit win: std.window.Window, read img: std.canvas.Image) -> Int:\n",
            "    let size = std.canvas.image_size :: img :: call\n",
            "    if size.0 != 16 or size.1 != 16:\n",
            "        return 1\n",
            "    std.canvas.blit :: win, img, 7 :: call\n",
            "        y = 8\n",
            "    std.canvas.blit_scaled :: win, img, 1 :: call\n",
            "        y = 2\n",
            "        w = 3\n",
            "        h = 4\n",
            "    std.canvas.blit_region :: win, img, 0 :: call\n",
            "        sy = 0\n",
            "        sw = 1\n",
            "        sh = 1\n",
            "        dx = 9\n",
            "        dy = 10\n",
            "        dw = 11\n",
            "        dh = 12\n",
            "    return 0\n",
            "fn run(take win: std.window.Window) -> Int:\n",
            "    let mut win = win\n",
            "    if not (std.window.alive :: win :: call):\n",
            "        return 2\n",
            "    let size = std.window.size :: win :: call\n",
            "    if size.0 != 320 or size.1 != 200:\n",
            "        return 3\n",
            "    std.window.set_title :: win, \"Renamed\" :: call\n",
            "    std.window.set_topmost :: win, true :: call\n",
            "    let color = std.canvas.rgb :: 10, 20, 30 :: call\n",
            "    let rect = std.canvas.RectSpec :: pos = (1, 2), size = (3, 4), color = color :: call\n",
            "    std.canvas.fill :: win, color :: call\n",
            "    std.canvas.rect_draw :: win, rect :: call\n",
            "    std.canvas.label :: win, 5, 6 :: call\n",
            "        text = \"Arcana\"\n",
            "        color = color\n",
            "    let label_size = std.canvas.label_size :: \"Arcana\" :: call\n",
            "    if label_size.0 <= 0:\n",
            "        return 4\n",
            "    let image_status = match (std.canvas.image_load :: \"sprite.bin\" :: call):\n",
            "        Result.Ok(img) => draw_image :: win, img :: call\n",
            "        Result.Err(_) => 5\n",
            "    if image_status != 0:\n",
            "        return 6\n",
            "    std.canvas.present :: win :: call\n",
            "    let start = std.time.monotonic_now_ms :: :: call\n",
            "    std.time.sleep_ms :: 5 :: call\n",
            "    let end = std.time.monotonic_now_ms :: :: call\n",
            "    let delta = std.time.elapsed_ms :: start, end :: call\n",
            "    if delta.value < 0:\n",
            "        return 7\n",
            "    let mut frame = std.events.pump :: win :: call\n",
            "    if not (std.input.mouse_in_window :: frame :: call):\n",
            "        return 8\n",
            "    if (std.input.mouse_pos :: frame :: call).0 != 40:\n",
            "        return 9\n",
            "    let key = std.input.key_code :: \"A\" :: call\n",
            "    if not (std.input.key_down :: frame, key :: call):\n",
            "        return 10\n",
            "    let first = std.events.poll :: frame :: call\n",
            "    if first :: :: is_none:\n",
            "        return 11\n",
            "    let second = std.events.poll :: frame :: call\n",
            "    if second :: :: is_none:\n",
            "        return 12\n",
            "    let none = std.events.poll :: frame :: call\n",
            "    if not (none :: :: is_none):\n",
            "        return 13\n",
            "    let close = std.window.close :: win :: call\n",
            "    if close :: :: is_err:\n",
            "        return 14\n",
            "    return 0\n",
            "fn main() -> Int:\n",
            "    return match (std.window.open :: \"Arcana\", 320, 200 :: call):\n",
            "        Result.Ok(win) => run :: win :: call\n",
            "        Result.Err(_) => 99\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let graph = load_workspace_graph(&dir).expect("workspace graph should load");
    let checked = check_workspace_graph(&graph).expect("workspace should check");
    let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
        .expect("fingerprints should compute");
    let order = plan_workspace(&graph).expect("workspace order should plan");
    let statuses =
        plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
    execute_build(&graph, &statuses).expect("build should execute");

    let artifact_path = graph.root_dir.join(
        &statuses
            .iter()
            .find(|status| status.member == "runtime_window_canvas")
            .expect("app artifact status should exist")
            .artifact_rel_path,
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let fixture_root = dir.join("fixture");
    let decode_routine = resolve_routine_index(
        &plan,
        &plan.root_module_id,
        &[
            "std".to_string(),
            "kernel".to_string(),
            "events".to_string(),
            "decode".to_string(),
        ],
    )
    .expect("std.kernel.events.decode should exist");
    let kernel_poll_routine = resolve_routine_index(
        &plan,
        &plan.root_module_id,
        &[
            "std".to_string(),
            "kernel".to_string(),
            "events".to_string(),
            "poll".to_string(),
        ],
    )
    .expect("std.kernel.events.poll should exist");
    let lift_event_routine = resolve_routine_index(
        &plan,
        &plan.root_module_id,
        &[
            "std".to_string(),
            "events".to_string(),
            "lift_event".to_string(),
        ],
    )
    .expect("std.events.lift_event should exist");
    let poll_routine = resolve_routine_index(
        &plan,
        &plan.root_module_id,
        &["std".to_string(), "events".to_string(), "poll".to_string()],
    )
    .expect("std.events.poll should exist");

    let mut debug_host = synthetic_window_canvas_host(&fixture_root);
    let decoded = execute_routine(
        &plan,
        decode_routine,
        vec![
            RuntimeValue::Int(3),
            RuntimeValue::Int(1),
            RuntimeValue::Int(0),
        ],
        &mut debug_host,
    )
    .expect("std.kernel.events.decode should execute");
    assert_eq!(
        decoded,
        RuntimeValue::Variant {
            name: "std.kernel.events.Event.WindowFocused".to_string(),
            payload: vec![RuntimeValue::Bool(true)],
        }
    );

    let debug_window = debug_host
        .window_open("Arcana", 320, 200)
        .expect("debug window should open");
    let debug_frame = debug_host
        .events_pump(debug_window)
        .expect("debug frame should pump");
    let kernel_polled = execute_routine(
        &plan,
        kernel_poll_routine,
        vec![RuntimeValue::Opaque(RuntimeOpaqueValue::AppFrame(
            debug_frame,
        ))],
        &mut debug_host,
    )
    .expect("std.kernel.events.poll should execute");
    assert_eq!(
        kernel_polled,
        RuntimeValue::Variant {
            name: "std.kernel.events.Event.WindowFocused".to_string(),
            payload: vec![RuntimeValue::Bool(true)],
        }
    );
    let lifted_direct = execute_routine(
        &plan,
        lift_event_routine,
        vec![kernel_polled.clone()],
        &mut debug_host,
    )
    .expect("std.events.lift_event should execute");
    assert_eq!(
        lifted_direct,
        RuntimeValue::Variant {
            name: "std.option.Option.Some".to_string(),
            payload: vec![RuntimeValue::Variant {
                name: "AppEvent.WindowFocused".to_string(),
                payload: vec![RuntimeValue::Bool(true)],
            }],
        }
    );

    let mut debug_host = synthetic_window_canvas_host(&fixture_root);
    let debug_window = debug_host
        .window_open("Arcana", 320, 200)
        .expect("debug window should open");
    let debug_frame = debug_host
        .events_pump(debug_window)
        .expect("debug frame should pump");
    let lifted = execute_routine(
        &plan,
        poll_routine,
        vec![RuntimeValue::Opaque(RuntimeOpaqueValue::AppFrame(
            debug_frame,
        ))],
        &mut debug_host,
    )
    .expect("std.events.poll should execute");
    assert_eq!(
        lifted,
        RuntimeValue::Variant {
            name: "std.option.Option.Some".to_string(),
            payload: vec![RuntimeValue::Variant {
                name: "AppEvent.WindowFocused".to_string(),
                payload: vec![RuntimeValue::Bool(true)],
            }],
        }
    );

    let mut host = synthetic_window_canvas_host(&fixture_root);
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(host.sleep_log_ms, vec![5]);
    assert_eq!(
        host.canvas_log,
        vec![
            "fill:660510".to_string(),
            "rect:1,2,3,4,660510".to_string(),
            "label:5,6,Arcana,660510".to_string(),
            format!(
                "blit:{}/sprite.bin,7,8",
                fixture_root.to_string_lossy().replace('\\', "/")
            ),
            format!(
                "blit_scaled:{}/sprite.bin,1,2,3,4",
                fixture_root.to_string_lossy().replace('\\', "/",)
            ),
            format!(
                "blit_region:{}/sprite.bin,0,0,1,1,9,10,11,12",
                fixture_root.to_string_lossy().replace('\\', "/",)
            ),
            "present".to_string(),
        ]
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_synthetic_host_core_workspace_artifact() {
    let dir = temp_workspace_dir("host_tool");
    write_host_core_workspace(&dir);

    let fixture_root = dir.join("fixture");
    write_file(&fixture_root.join("alpha.arc"), "alpha");
    write_file(&fixture_root.join("notes.txt"), "skip me");

    let graph = load_workspace_graph(&dir).expect("workspace graph should load");
    let checked = check_workspace_graph(&graph).expect("workspace should check");
    let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
        .expect("fingerprints should compute");
    let order = plan_workspace(&graph).expect("workspace order should plan");
    let statuses =
        plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
    execute_build(&graph, &statuses).expect("build should execute");

    let artifact_path = graph.root_dir.join(
        &statuses
            .iter()
            .find(|status| status.member == "runtime_host_core")
            .expect("app artifact status should exist")
            .artifact_rel_path,
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");

    let cwd = fixture_root.to_string_lossy().replace('\\', "/");
    let mut host = BufferedHost {
        cwd: cwd.clone(),
        sandbox_root: cwd.clone(),
        ..BufferedHost::default()
    };
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(
        host.stdout,
        vec![
            format!("{cwd}/alpha.arc"),
            "1".to_string(),
            "12".to_string(),
        ]
    );

    let report_path = fixture_root
        .join(".arcana")
        .join("logs")
        .join("host_core_report.txt");
    assert_eq!(
        fs::read_to_string(&report_path).expect("report should write"),
        "Arcana Runtime Host Core v1\n"
    );

    let _ = fs::remove_dir_all(dir);
}
