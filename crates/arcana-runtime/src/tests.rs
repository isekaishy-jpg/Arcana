use super::{
    BufferedHost, ParsedAssignOp, ParsedAssignTarget, ParsedCleanupFooter, ParsedExpr,
    ParsedPhraseArg, ParsedPhraseQualifierKind, ParsedProjectionFamily, ParsedStmt,
    RuntimeBindingOpaqueValue, RuntimeCallArg, RuntimeEntrypointPlan, RuntimeExecutionState,
    RuntimeFrameArenaHandle, RuntimeFrameArenaPolicy, RuntimeFrameArenaState,
    RuntimeFrameRecyclePolicy, RuntimeIntrinsic, RuntimeLocal, RuntimeLocalHandle,
    RuntimeMemoryHandlePolicy, RuntimeMemoryPressurePolicy, RuntimeNativeCallbackPlan,
    RuntimeOpaqueValue, RuntimePackagePlan, RuntimeParamPlan, RuntimePoolIdValue,
    RuntimeReferenceMode, RuntimeReferenceTarget, RuntimeReferenceValue, RuntimeResetOnPolicy,
    RuntimeRingIdValue, RuntimeRoutinePlan, RuntimeScope, RuntimeSessionIdValue,
    RuntimeSlabIdValue, RuntimeSlabPolicy, RuntimeSlabState, RuntimeTempArenaHandle,
    RuntimeTypeBindings, RuntimeValue, default_runtime_pool_policy, default_runtime_ring_policy,
    default_runtime_session_policy, default_runtime_slab_policy, ensure_runtime_frame_capacity,
    ensure_runtime_slab_capacity, execute_entrypoint_routine, execute_exported_abi_routine,
    execute_exported_json_abi_routine, execute_main, execute_routine, execute_routine_with_state,
    execute_runtime_intrinsic, insert_runtime_channel, insert_runtime_local,
    insert_runtime_pool_arena, insert_runtime_read_view_from_reference,
    insert_runtime_read_view_from_ring_window, insert_runtime_ring_buffer,
    insert_runtime_session_arena, insert_runtime_slab, load_package_plan,
    lookup_runtime_owner_plan, owner_state_key, parse_cleanup_footer_row,
    parse_runtime_package_image, parse_stmt, plan_from_artifact, pool_id_is_live,
    read_runtime_reference, reclaim_held_target_local, reclaim_hold_capability_root_local,
    redeem_take_reference, render_exported_json_abi_manifest, render_runtime_package_image,
    resolve_routine_index, resolve_routine_index_for_call, runtime_read_view_snapshot,
    runtime_reset_owner_exit_memory_specs_in_scopes, runtime_validate_split_value,
    validate_scope_hold_tokens, write_assign_target_value_runtime,
};
use arcana_aot::{
    AOT_INTERNAL_FORMAT, AotEntrypointArtifact, AotOwnerArtifact, AotPackageArtifact,
    AotPackageModuleArtifact, AotRoutineArtifact, AotRoutineParamArtifact, render_package_artifact,
};
use arcana_frontend::{check_workspace_graph, compute_member_fingerprints_for_checked_workspace};
use arcana_ir::{
    ExecMemorySpecDecl, IrForewordArg, IrForewordMetadata, IrForewordRetention, IrRoutineType,
    IrRoutineTypeKind, parse_routine_type_text,
};
use arcana_package::{execute_build, load_workspace_graph, plan_workspace, prepare_build};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

trait TestParamRow: Sized {
    fn from_test_row(row: &str) -> Self;
}

impl TestParamRow for AotRoutineParamArtifact {
    fn from_test_row(row: &str) -> Self {
        let parts = row.splitn(3, ':').collect::<Vec<_>>();
        let mode = parts[0].strip_prefix("mode=").unwrap_or_default();
        let name = parts[1].strip_prefix("name=").unwrap_or_default();
        let ty = parts[2].strip_prefix("ty=").unwrap_or_default();
        Self {
            binding_id: 0,
            mode: (!mode.is_empty()).then(|| mode.to_string()),
            name: name.to_string(),
            ty: parse_routine_type_text(ty).expect("type should parse"),
        }
    }
}

fn test_params<T, S>(rows: &[S]) -> Vec<T>
where
    T: TestParamRow,
    S: AsRef<str>,
{
    rows.iter()
        .map(|row| T::from_test_row(row.as_ref()))
        .collect()
}

fn test_return_type(signature: &str) -> Option<IrRoutineType> {
    let (_, tail) = signature.rsplit_once("->")?;
    let trimmed = tail.trim().trim_end_matches(':').trim();
    (!trimmed.is_empty()).then(|| parse_routine_type_text(trimmed).expect("type should parse"))
}

fn test_package_id_for_module(module_id: &str) -> String {
    module_id.split('.').next().unwrap_or(module_id).to_string()
}

fn empty_runtime_plan(package_id: &str) -> RuntimePackagePlan {
    RuntimePackagePlan {
        package_id: package_id.to_string(),
        package_name: package_id.to_string(),
        root_module_id: package_id.to_string(),
        direct_deps: Vec::new(),
        direct_dep_ids: Vec::new(),
        package_display_names: test_package_display_names_with_deps(
            package_id.to_string(),
            package_id.to_string(),
            Vec::new(),
            Vec::new(),
        ),
        package_direct_dep_ids: test_package_direct_dep_ids(
            package_id.to_string(),
            Vec::new(),
            Vec::new(),
        ),
        runtime_requirements: Vec::new(),
        foreword_index: Vec::new(),
        foreword_registrations: Vec::new(),
        module_aliases: BTreeMap::new(),
        opaque_family_types: BTreeMap::new(),
        entrypoints: Vec::new(),
        routines: Vec::new(),
        native_callbacks: Vec::new(),
        shackle_decls: Vec::new(),
        binding_layouts: Vec::new(),
        owners: Vec::new(),
    }
}

fn test_package_display_names_with_deps(
    package_id: impl Into<String>,
    package_name: impl Into<String>,
    direct_deps: Vec<String>,
    direct_dep_ids: Vec<String>,
) -> BTreeMap<String, String> {
    let mut names = BTreeMap::from([(package_id.into(), package_name.into())]);
    for (dep_name, dep_id) in direct_deps.into_iter().zip(direct_dep_ids) {
        names.entry(dep_id).or_insert(dep_name);
    }
    names
}

fn test_package_direct_dep_ids(
    package_id: impl Into<String>,
    direct_deps: Vec<String>,
    direct_dep_ids: Vec<String>,
) -> BTreeMap<String, BTreeMap<String, String>> {
    BTreeMap::from([(
        package_id.into(),
        direct_deps.into_iter().zip(direct_dep_ids).collect(),
    )])
}

fn temp_artifact_path(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should advance")
        .as_nanos();
    let path = repo_root()
        .join("target")
        .join("arcana-runtime-tests")
        .join(format!("{label}_{nanos}.toml"));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("runtime temp dir should exist");
    }
    path
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

fn maybe_inject_test_grimoire_deps(path: &Path, text: &str) -> String {
    if path.file_name().and_then(|name| name.to_str()) != Some("book.toml") {
        return text.to_string();
    }
    let Some(manifest_dir) = path.parent() else {
        return text.to_string();
    };
    let Ok(mut value) = text.parse::<toml::Value>() else {
        return text.to_string();
    };
    let Some(table) = value.as_table_mut() else {
        return text.to_string();
    };
    if !table.contains_key("name") || !table.contains_key("kind") {
        return text.to_string();
    }

    let deps = table
        .entry("deps")
        .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));
    let Some(deps_table) = deps.as_table_mut() else {
        return text.to_string();
    };

    for (name, repo_relative) in [("arcana_process", "grimoires/arcana/process")] {
        if deps_table.contains_key(name) {
            continue;
        }
        let dep_path = repo_root().join(repo_relative);
        let Some(relative) = pathdiff::diff_paths(&dep_path, manifest_dir) else {
            continue;
        };
        let mut dep = toml::map::Map::new();
        dep.insert(
            "path".to_string(),
            toml::Value::String(relative.to_string_lossy().replace('\\', "/")),
        );
        deps_table.insert(name.to_string(), toml::Value::Table(dep));
    }

    toml::to_string(&value).unwrap_or_else(|_| text.to_string())
}

fn write_file(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("parent directory should exist");
    }
    let rendered = maybe_inject_test_grimoire_deps(path, text);
    fs::write(path, rendered).expect("test file should write");
}

fn plan_build(
    graph: &arcana_package::WorkspaceGraph,
    order: &[String],
    _fingerprints: &arcana_package::WorkspaceFingerprints,
    existing_lock: Option<&arcana_package::Lockfile>,
) -> Result<Vec<arcana_package::BuildStatus>, String> {
    let prepared = prepare_build(graph)?;
    arcana_package::plan_build(graph, order, &prepared, existing_lock)
}

fn execute_workspace_build(
    graph: &arcana_package::WorkspaceGraph,
    _fingerprints: &arcana_package::WorkspaceFingerprints,
    statuses: &[arcana_package::BuildStatus],
) {
    let prepared = prepare_build(graph).expect("prepare build");
    execute_build(graph, &prepared, statuses).expect("workspace build should execute");
}

fn build_workspace_plan_for_member(dir: &Path, member: &str) -> RuntimePackagePlan {
    let graph = load_workspace_graph(dir).expect("workspace graph should load");
    let checked = check_workspace_graph(&graph).expect("workspace should check");
    let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
        .expect("fingerprints should compute");
    let order = plan_workspace(&graph).expect("workspace order should plan");
    let statuses =
        plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
    execute_workspace_build(&graph, &fingerprints, &statuses);
    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == member)
            .expect("member artifact status should exist")
            .artifact_rel_path(),
    );
    load_package_plan(&artifact_path).expect("runtime plan should load")
}

#[cfg(unix)]
fn create_test_symlink_file(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(windows)]
fn create_test_symlink_file(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_file(target, link)
}

#[cfg(unix)]
fn create_test_symlink_dir(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(windows)]
fn create_test_symlink_dir(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_dir(target, link)
}

fn repo_root() -> PathBuf {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .parent()
        .and_then(Path::parent)
        .expect("workspace root should exist")
        .to_path_buf()
}

#[test]
fn buffered_host_sandbox_rejects_symlink_file_escape() {
    let dir = temp_workspace_dir("sandbox_symlink_file_escape");
    let sandbox = dir.join("sandbox");
    let outside = dir.join("outside");
    fs::create_dir_all(&sandbox).expect("sandbox root should exist");
    fs::create_dir_all(&outside).expect("outside dir should exist");
    fs::write(outside.join("secret.txt"), "arcana").expect("outside file should exist");

    let link = sandbox.join("secret.txt");
    match create_test_symlink_file(&outside.join("secret.txt"), &link) {
        Ok(()) => {}
        Err(err)
            if err.kind() == std::io::ErrorKind::PermissionDenied
                || err.raw_os_error() == Some(1314) =>
        {
            let _ = fs::remove_dir_all(dir);
            return;
        }
        Err(err) => panic!("symlink should create: {err}"),
    }

    let sandbox_text = sandbox.to_string_lossy().replace('\\', "/");
    let host = BufferedHost {
        cwd: sandbox_text.clone(),
        sandbox_root: sandbox_text,
        ..BufferedHost::default()
    };

    let err = host
        .fs_read_text("secret.txt")
        .expect_err("sandbox should reject file symlink escape");
    assert!(err.contains("escapes sandbox root"), "{err}");

    let err = host
        .path_canonicalize("secret.txt")
        .expect_err("sandbox canonicalize should reject file symlink escape");
    assert!(err.contains("escapes sandbox root"), "{err}");

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn buffered_host_sandbox_rejects_symlink_parent_escape_for_write() {
    let dir = temp_workspace_dir("sandbox_symlink_parent_escape");
    let sandbox = dir.join("sandbox");
    let outside = dir.join("outside");
    fs::create_dir_all(&sandbox).expect("sandbox root should exist");
    fs::create_dir_all(&outside).expect("outside dir should exist");

    let link = sandbox.join("shared");
    match create_test_symlink_dir(&outside, &link) {
        Ok(()) => {}
        Err(err)
            if err.kind() == std::io::ErrorKind::PermissionDenied
                || err.raw_os_error() == Some(1314) =>
        {
            let _ = fs::remove_dir_all(dir);
            return;
        }
        Err(err) => panic!("directory symlink should create: {err}"),
    }

    let sandbox_text = sandbox.to_string_lossy().replace('\\', "/");
    let host = BufferedHost {
        cwd: sandbox_text.clone(),
        sandbox_root: sandbox_text,
        ..BufferedHost::default()
    };

    let err = host
        .fs_write_text("shared/escape.txt", "blocked")
        .expect_err("sandbox should reject write through symlinked parent");
    assert!(err.contains("escapes sandbox root"), "{err}");
    assert!(
        !outside.join("escape.txt").exists(),
        "write-through symlink must not create outside files"
    );

    let _ = fs::remove_dir_all(dir);
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
            "import arcana_process.fs\n",
            "import arcana_process.io\n",
            "import arcana_process.path\n",
            "import std.text\n",
            "use std.result.Result\n",
            "\n",
            "fn list_arc_files(root: Str) -> List[Str]:\n",
            "    let mut pending = std.collections.list.new[Str] :: :: call\n",
            "    let mut files = std.collections.list.new[Str] :: :: call\n",
            "    pending :: root :: push\n",
            "    while (pending :: :: len) > 0:\n",
            "        let path = pending :: :: pop\n",
            "        if arcana_process.fs.is_dir :: path :: call:\n",
            "            let mut entries = match (arcana_process.fs.list_dir :: path :: call):\n",
            "                Result.Ok(found) => found\n",
            "                Result.Err(_) => std.collections.list.new[Str] :: :: call\n",
            "            while (entries :: :: len) > 0:\n",
            "                pending :: (entries :: :: pop) :: push\n",
            "            continue\n",
            "        if (arcana_process.path.ext :: path :: call) != \"arc\":\n",
            "            continue\n",
            "        files :: path :: push\n",
            "    return files\n",
            "\n",
            "fn read_text_or_empty(path: Str) -> Str:\n",
            "    return match (arcana_process.fs.read_text :: path :: call):\n",
            "        Result.Ok(text) => text\n",
            "        Result.Err(_) => \"\"\n",
            "\n",
            "fn main() -> Int:\n",
            "    let root = arcana_process.path.cwd :: :: call\n",
            "    let mut files = list_arc_files :: root :: call\n",
            "    let mut count = 0\n",
            "    let mut checksum = 0\n",
            "    while (files :: :: len) > 0:\n",
            "        let file = files :: :: pop\n",
            "        let text = read_text_or_empty :: file :: call\n",
            "        let size = std.text.len_bytes :: text :: call\n",
            "        arcana_process.io.print[Str] :: file :: call\n",
            "        count += 1\n",
            "        checksum = ((checksum * 131) + size + 7) % 2147483647\n",
            "    let report_dir = arcana_process.path.join :: root, \".arcana\" :: call\n",
            "    let logs_dir = arcana_process.path.join :: report_dir, \"logs\" :: call\n",
            "    let report_path = arcana_process.path.join :: logs_dir, \"host_core_report.txt\" :: call\n",
            "    arcana_process.fs.mkdir_all :: logs_dir :: call\n",
            "    arcana_process.fs.write_text :: report_path, \"Arcana Runtime Host Core v1\\n\" :: call\n",
            "    arcana_process.io.print[Int] :: count :: call\n",
            "    arcana_process.io.print[Int] :: checksum :: call\n",
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
        format: AOT_INTERNAL_FORMAT.to_string(),
        package_id: "hello".to_string(),
        package_name: "hello".to_string(),
        root_module_id: "hello".to_string(),
        direct_deps: vec!["std".to_string()],
        direct_dep_ids: vec!["std".to_string()],
        package_display_names: test_package_display_names_with_deps(
            "hello".to_string(),
            "hello".to_string(),
            vec!["std".to_string()],
            vec!["std".to_string()],
        ),
        package_direct_dep_ids: test_package_direct_dep_ids(
            test_package_id_for_module("hello"),
            vec!["std".to_string()],
            vec!["std".to_string()],
        ),
        module_count: 1,
        dependency_edge_count: 1,
        dependency_rows: vec!["source=hello:import:arcana_process.io:".to_string()],
        exported_surface_rows: vec!["module=hello:export:fn:fn main() -> Int:".to_string()],
        runtime_requirements: vec!["arcana_process.io".to_string()],
        foreword_index: Vec::new(),
        foreword_registrations: Vec::new(),
        entrypoints: vec![AotEntrypointArtifact {
            package_id: test_package_id_for_module("hello"),
            module_id: "hello".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            is_async: false,
            exported: true,
        }],
        routines: vec![AotRoutineArtifact {
            package_id: test_package_id_for_module("hello"),
            module_id: "hello".to_string(),
            routine_key: "hello#sym-0".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            exported: true,
            is_async: false,
            type_params: Vec::new(),
            behavior_attrs: BTreeMap::new(),
            params: Vec::new(),
            return_type: test_return_type("fn main() -> Int:"),
            intrinsic_impl: None,
            native_impl: None,
            impl_target_type: None,
            impl_trait_path: None,
            availability: Vec::new(),
            cleanup_footers: Vec::new(),
            statements: vec![
                parse_stmt("stmt(core=return(int(7)),forewords=[],cleanup_footers=[])")
                    .expect("statement should parse"),
            ],
        }],
        native_callbacks: Vec::new(),
        shackle_decls: Vec::new(),
        binding_layouts: Vec::new(),
        owners: Vec::new(),
        modules: vec![AotPackageModuleArtifact {
            package_id: test_package_id_for_module("hello"),
            module_id: "hello".to_string(),
            symbol_count: 1,
            item_count: 2,
            line_count: 2,
            non_empty_line_count: 2,
            directive_rows: vec!["module=hello:import:arcana_process.io:".to_string()],
            lang_item_rows: Vec::new(),
            exported_surface_rows: vec!["export:fn:fn main() -> Int:".to_string()],
        }],
    }
}

fn sample_print_artifact() -> AotPackageArtifact {
    AotPackageArtifact {
            format: AOT_INTERNAL_FORMAT.to_string(),
            package_id: "hello".to_string(),
            package_name: "hello".to_string(),
            root_module_id: "hello".to_string(),
            direct_deps: vec!["std".to_string()],
            direct_dep_ids: vec!["std".to_string()],
            package_display_names: test_package_display_names_with_deps("hello".to_string(), "hello".to_string(), vec!["std".to_string()], vec!["std".to_string()]),
            package_direct_dep_ids: test_package_direct_dep_ids(test_package_id_for_module("hello"), vec!["std".to_string()], vec!["std".to_string()]),
            module_count: 1,
            dependency_edge_count: 2,
            dependency_rows: vec![
                "source=hello:import:arcana_process.io:".to_string(),
                "source=hello:use:arcana_process.io:io".to_string(),
            ],
            exported_surface_rows: vec![],
            runtime_requirements: vec!["arcana_process.io".to_string()],
            foreword_index: Vec::new(),
            foreword_registrations: Vec::new(),
            entrypoints: vec![AotEntrypointArtifact {
                package_id: test_package_id_for_module("hello"),
                module_id: "hello".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                is_async: false,
                exported: false,
            }],
            routines: vec![AotRoutineArtifact {
                package_id: test_package_id_for_module("hello"),
                module_id: "hello".to_string(),
                routine_key: "hello#sym-0".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                exported: false,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: Vec::new(),
                return_type: test_return_type("fn main():"),
                intrinsic_impl: None,
            native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                cleanup_footers: Vec::new(),
                statements: vec![parse_stmt("stmt(core=expr(phrase(subject=generic(expr=member(path(io), print),types=[Str]),args=[str(\"\\\"hello, arcana\\\"\")],qualifier=call,attached=[])),forewords=[],cleanup_footers=[])")
                    .expect("statement should parse")],
            }],
            native_callbacks: Vec::new(),
            shackle_decls: Vec::new(),
        binding_layouts: Vec::new(),
        owners: Vec::new(),
            modules: vec![AotPackageModuleArtifact {
                package_id: test_package_id_for_module("hello"),
                module_id: "hello".to_string(),
                symbol_count: 1,
                item_count: 4,
                line_count: 4,
                non_empty_line_count: 4,
                directive_rows: vec![
                    "module=hello:import:arcana_process.io:".to_string(),
                    "module=hello:use:arcana_process.io:io".to_string(),
                ],
                lang_item_rows: Vec::new(),
                exported_surface_rows: Vec::new(),
            }],
        }
}

fn sample_stmt_metadata_artifact() -> AotPackageArtifact {
    AotPackageArtifact {
            format: AOT_INTERNAL_FORMAT.to_string(),
            package_id: "metadata".to_string(),
            package_name: "metadata".to_string(),
            root_module_id: "metadata".to_string(),
            direct_deps: Vec::new(),
            direct_dep_ids: Vec::new(),
            package_display_names: test_package_display_names_with_deps("metadata".to_string(), "metadata".to_string(), Vec::new(), Vec::new()),
            package_direct_dep_ids: test_package_direct_dep_ids(test_package_id_for_module("metadata"), Vec::new(), Vec::new()),
            module_count: 1,
            dependency_edge_count: 0,
            dependency_rows: Vec::new(),
            exported_surface_rows: vec!["module=metadata:export:fn:fn main() -> Int:".to_string()],
            runtime_requirements: Vec::new(),
            foreword_index: Vec::new(),
            foreword_registrations: Vec::new(),
            entrypoints: vec![AotEntrypointArtifact {
                package_id: test_package_id_for_module("metadata"),
                module_id: "metadata".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                is_async: false,
                exported: true,
            }],
            routines: vec![
                AotRoutineArtifact {
                    package_id: test_package_id_for_module("metadata"),
                module_id: "metadata".to_string(),
                    routine_key: "metadata#sym-0".to_string(),
                    symbol_name: "main".to_string(),
                    symbol_kind: "fn".to_string(),
                    exported: true,
                    is_async: false,
                    type_params: Vec::new(),
                    behavior_attrs: BTreeMap::new(),
                    params: Vec::new(),
                    return_type: test_return_type("fn main() -> Int:"),
                    intrinsic_impl: None,
            native_impl: None,
                    impl_target_type: None,
                    impl_trait_path: None,
                    availability: Vec::new(),
                    cleanup_footers: vec![parse_cleanup_footer_row("cleanup:scope:metadata.cleanup")
                        .expect("cleanup footer should parse")],
                    statements: vec![parse_stmt(
                        "stmt(core=return(int(0)),forewords=[only(os=\"windows\")],cleanup_footers=[cleanup:scope:metadata.cleanup])",
                    )
                    .expect("statement should parse")],
                },
                AotRoutineArtifact {
                    package_id: test_package_id_for_module("metadata"),
                module_id: "metadata".to_string(),
                    routine_key: "metadata#sym-1".to_string(),
                    symbol_name: "cleanup".to_string(),
                    symbol_kind: "fn".to_string(),
                    exported: false,
                    is_async: false,
                    type_params: Vec::new(),
                    behavior_attrs: BTreeMap::new(),
                    params: test_params(&["mode=take:name=scope:ty=Int".to_string()]),
                    return_type: test_return_type("fn cleanup(take scope: Int) -> Result[Unit, Str]:"),
                    intrinsic_impl: None,
            native_impl: None,
                    impl_target_type: None,
                    impl_trait_path: None,
                    availability: Vec::new(),
                    cleanup_footers: Vec::new(),
                    statements: vec![parse_stmt("stmt(core=return(phrase(subject=generic(expr=path(Result.Ok),types=[Unit,Str]),args=[],qualifier=call,attached=[])),forewords=[],cleanup_footers=[])")
                        .expect("statement should parse")],
                },
            ],
            native_callbacks: Vec::new(),
            shackle_decls: Vec::new(),
        binding_layouts: Vec::new(),
        owners: Vec::new(),
            modules: vec![AotPackageModuleArtifact {
                package_id: test_package_id_for_module("metadata"),
                module_id: "metadata".to_string(),
                symbol_count: 2,
                item_count: 2,
                line_count: 1,
                non_empty_line_count: 1,
                directive_rows: Vec::new(),
                lang_item_rows: Vec::new(),
                exported_surface_rows: vec!["export:fn:fn main() -> Int:".to_string()],
            }],
        }
}

fn sample_attachment_foreword_artifact() -> AotPackageArtifact {
    AotPackageArtifact {
            format: AOT_INTERNAL_FORMAT.to_string(),
            package_id: "attachment".to_string(),
            package_name: "attachment".to_string(),
            root_module_id: "attachment".to_string(),
            direct_deps: vec!["std".to_string()],
            direct_dep_ids: vec!["std".to_string()],
            package_display_names: test_package_display_names_with_deps("attachment".to_string(), "attachment".to_string(), vec!["std".to_string()], vec!["std".to_string()]),
            package_direct_dep_ids: test_package_direct_dep_ids(test_package_id_for_module("attachment"), vec!["std".to_string()], vec!["std".to_string()]),
            module_count: 1,
            dependency_edge_count: 0,
            dependency_rows: Vec::new(),
            exported_surface_rows: vec![
                "module=attachment:export:fn:fn main() -> Int:".to_string(),
            ],
            runtime_requirements: vec!["arcana_process.io".to_string()],
            foreword_index: Vec::new(),
            foreword_registrations: Vec::new(),
            entrypoints: vec![AotEntrypointArtifact {
                package_id: test_package_id_for_module("attachment"),
                module_id: "attachment".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                is_async: false,
                exported: true,
            }],
            routines: vec![AotRoutineArtifact {
                package_id: test_package_id_for_module("attachment"),
                module_id: "attachment".to_string(),
                routine_key: "attachment#sym-0".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: Vec::new(),
                return_type: test_return_type("fn main() -> Int:"),
                intrinsic_impl: None,
            native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                cleanup_footers: Vec::new(),
                statements: vec![
                    parse_stmt(
                        "stmt(core=let(mutable=true,name=xs,value=collection([int(1)])),forewords=[],cleanup_footers=[])",
                    )
                    .expect("statement should parse"),
                    parse_stmt(
                        "stmt(core=expr(phrase(subject=path(std.kernel.collections.list_push),args=[path(xs)],kind=call,qualifier=call,attached=[chain(int(2),forewords=[inline()])])),forewords=[],cleanup_footers=[])",
                    )
                    .expect("statement should parse"),
                    parse_stmt(
                        "stmt(core=expr(phrase(subject=generic(expr=path(arcana_process.io.print),types=[Int]),args=[phrase(subject=path(std.kernel.collections.list_len),args=[path(xs)],kind=call,qualifier=call,attached=[])],kind=call,qualifier=call,attached=[])),forewords=[],cleanup_footers=[])",
                    )
                    .expect("statement should parse"),
                    parse_stmt("stmt(core=return(int(0)),forewords=[],cleanup_footers=[])")
                        .expect("statement should parse"),
                ],
            }],
            native_callbacks: Vec::new(),
            shackle_decls: Vec::new(),
        binding_layouts: Vec::new(),
        owners: Vec::new(),
            modules: vec![AotPackageModuleArtifact {
                package_id: test_package_id_for_module("attachment"),
                module_id: "attachment".to_string(),
                symbol_count: 1,
                item_count: 4,
                line_count: 4,
                non_empty_line_count: 4,
                directive_rows: Vec::new(),
                lang_item_rows: Vec::new(),
                exported_surface_rows: vec!["export:fn:fn main() -> Int:".to_string()],
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
    assert_eq!(
        plan.runtime_requirements,
        vec!["arcana_process.io".to_string()]
    );
    let _ = fs::remove_file(path);
}

#[test]
fn runtime_package_image_roundtrips_runtime_plan() {
    let plan = plan_from_artifact(&sample_return_artifact()).expect("runtime plan should build");
    let image = render_runtime_package_image(&plan).expect("runtime package image should render");
    let decoded = parse_runtime_package_image(&image).expect("runtime package image should parse");
    assert_eq!(decoded, plan);
}

#[test]
fn load_package_plan_accepts_behavior_attr_values_with_colons() {
    let dir = temp_workspace_dir("behavior_attr_colons");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_behavior_attr\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "behavior[phase=update:late] fn tick():\n",
            "    return 0\n",
            "fn main() -> Int:\n",
            "    return 0\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let graph = load_workspace_graph(&dir).expect("workspace graph should load");
    eprintln!("[asset-test] graph loaded");
    let checked = check_workspace_graph(&graph).expect("workspace should check");
    eprintln!("[asset-test] workspace checked");
    let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
        .expect("fingerprints should compute");
    eprintln!("[asset-test] fingerprints computed");
    let order = plan_workspace(&graph).expect("workspace order should plan");
    eprintln!("[asset-test] workspace planned");
    let statuses =
        plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
    eprintln!("[asset-test] build planned");
    execute_workspace_build(&graph, &fingerprints, &statuses);
    eprintln!("[asset-test] workspace built");

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_behavior_attr")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let tick = plan
        .routines
        .iter()
        .find(|routine| routine.symbol_name == "tick")
        .expect("behavior routine should be present");
    assert_eq!(
        tick.behavior_attrs.get("phase").map(String::as_str),
        Some("update:late")
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn plan_from_artifact_rejects_main_with_parameters() {
    let mut artifact = sample_return_artifact();
    artifact.routines[0].params = test_params(&["mode=:name=x:ty=Int".to_string()]);

    let err = plan_from_artifact(&artifact).expect_err("parameterized main should fail");
    assert!(
        err.contains("main must not take parameters in the current runtime lane"),
        "{err}"
    );
}

#[test]
fn plan_from_artifact_rejects_main_with_non_runtime_return_type() {
    let mut artifact = sample_return_artifact();
    artifact.routines[0].return_type = Some(parse_routine_type_text("Bool").expect("type"));

    let err = plan_from_artifact(&artifact).expect_err("bool-returning main should fail");
    assert!(
        err.contains("main must return Int or Unit in the current runtime lane"),
        "{err}"
    );
}

#[test]
fn plan_from_artifact_rejects_async_cleanup_footer_handler() {
    let mut artifact = sample_stmt_metadata_artifact();
    artifact.routines[1].is_async = true;

    let err = plan_from_artifact(&artifact).expect_err("async cleanup footer handler should fail");
    assert!(
        err.contains("cleanup footer handler `metadata.cleanup` cannot be async in v1"),
        "{err}"
    );
}

#[test]
fn plan_from_artifact_rejects_wrong_arity_cleanup_footer_handler() {
    let mut artifact = sample_stmt_metadata_artifact();
    artifact.routines[1].params.clear();

    let err =
        plan_from_artifact(&artifact).expect_err("wrong-arity cleanup footer handler should fail");
    assert!(
        err.contains(
            "cleanup footer handler `metadata.cleanup` must accept exactly one parameter in v1"
        ),
        "{err}"
    );
}

#[test]
fn plan_from_artifact_rejects_non_take_cleanup_footer_handler() {
    let mut artifact = sample_stmt_metadata_artifact();
    artifact.routines[1].params[0].mode = Some("read".to_string());

    let err =
        plan_from_artifact(&artifact).expect_err("non-take cleanup footer handler should fail");
    assert!(
        err.contains(
            "cleanup footer handler `metadata.cleanup` must take its target parameter in v1"
        ),
        "{err}"
    );
}

#[test]
fn plan_from_artifact_rejects_wrong_cleanup_footer_handler_return_type() {
    let mut artifact = sample_stmt_metadata_artifact();
    artifact.routines[1].params[0].mode = Some("take".to_string());
    artifact.routines[1].return_type = Some(parse_routine_type_text("Int").expect("type"));

    let err = plan_from_artifact(&artifact)
        .expect_err("wrong-returning cleanup footer handler should fail");
    assert!(
        err.contains(
            "cleanup footer handler `metadata.cleanup` must return `Result[Unit, Str]` in v1"
        ),
        "{err}"
    );
}

#[test]
fn plan_from_artifact_rejects_cleanup_footer_handler_outside_module_scope() {
    let mut artifact = sample_stmt_metadata_artifact();
    artifact.module_count = 2;
    artifact.routines[0].cleanup_footers = vec![
        parse_cleanup_footer_row("cleanup:scope:cleanup").expect("cleanup footer should parse"),
    ];
    artifact.routines[0].statements = vec![parse_stmt(
        "stmt(core=return(int(0)),forewords=[only(os=\"windows\")],cleanup_footers=[cleanup:scope:cleanup])",
    )
    .expect("statement should parse")];
    artifact.routines[1].module_id = "helpers".to_string();
    artifact.modules[0].symbol_count = 1;
    artifact.modules.push(AotPackageModuleArtifact {
        package_id: artifact.package_id.clone(),
        module_id: "helpers".to_string(),
        symbol_count: 1,
        item_count: 1,
        line_count: 1,
        non_empty_line_count: 1,
        directive_rows: Vec::new(),
        lang_item_rows: Vec::new(),
        exported_surface_rows: Vec::new(),
    });

    let err =
        plan_from_artifact(&artifact).expect_err("out-of-scope cleanup footer handler should fail");
    assert!(
        err.contains("cleanup footer handler `cleanup` does not resolve to a callable path"),
        "{err}"
    );
}

#[test]
fn plan_from_artifact_rejects_inconsistent_module_count() {
    let mut artifact = sample_return_artifact();
    artifact.module_count = 2;

    let err = plan_from_artifact(&artifact).expect_err("inconsistent module count should fail");
    assert!(
        err.contains("module_count=2 does not match modules.len()=1"),
        "{err}"
    );
}

#[test]
fn resolve_routine_index_for_call_prefers_lowered_routine_identity() {
    let plan = RuntimePackagePlan {
        package_id: "ops".to_string(),
        package_name: "ops".to_string(),
        root_module_id: "ops".to_string(),
        direct_deps: Vec::new(),
        direct_dep_ids: Vec::new(),
        package_display_names: test_package_display_names_with_deps(
            "ops".to_string(),
            "ops".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        package_direct_dep_ids: test_package_direct_dep_ids(
            "ops".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        runtime_requirements: Vec::new(),
        foreword_index: Vec::new(),
        foreword_registrations: Vec::new(),
        module_aliases: BTreeMap::new(),
        opaque_family_types: BTreeMap::new(),
        entrypoints: Vec::new(),
        native_callbacks: Vec::new(),
        shackle_decls: Vec::new(),
        binding_layouts: Vec::new(),
        owners: Vec::new(),
        routines: vec![
            RuntimeRoutinePlan {
                package_id: test_package_id_for_module("ops"),
                module_id: "ops".to_string(),
                routine_key: "ops#impl-0-method-0".to_string(),
                symbol_name: "load".to_string(),
                symbol_kind: "fn".to_string(),
                exported: false,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: vec![RuntimeParamPlan {
                    binding_id: 0,
                    mode: Some("read".to_string()),
                    name: "self".to_string(),
                    ty: parse_routine_type_text("AtomicInt").expect("type"),
                }],
                return_type: test_return_type("fn load(read self: AtomicInt) -> Int:"),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                cleanup_footers: Vec::new(),
                statements: Vec::new(),
            },
            RuntimeRoutinePlan {
                package_id: test_package_id_for_module("ops"),
                module_id: "ops".to_string(),
                routine_key: "ops#impl-1-method-0".to_string(),
                symbol_name: "load".to_string(),
                symbol_kind: "fn".to_string(),
                exported: false,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: vec![RuntimeParamPlan {
                    binding_id: 0,
                    mode: Some("read".to_string()),
                    name: "self".to_string(),
                    ty: parse_routine_type_text("AtomicBool").expect("type"),
                }],
                return_type: test_return_type("fn load(read self: AtomicBool) -> Bool:"),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                cleanup_footers: Vec::new(),
                statements: Vec::new(),
            },
        ],
    };

    let index = resolve_routine_index_for_call(
        &plan,
        "ops",
        "ops",
        &["ops".to_string(), "load".to_string()],
        &[RuntimeCallArg {
            name: None,
            value: RuntimeValue::Bool(true),
            source_expr: ParsedExpr::Bool(true),
        }],
        Some("ops#impl-0-method-0"),
        None,
        false,
        None,
    )
    .expect("lowered routine identity should resolve")
    .expect("call should resolve");

    assert_eq!(index, 0);
}

#[test]
fn runtime_dynamic_bare_method_fallback_matches_receiver_type_args() {
    let plan = RuntimePackagePlan {
        package_id: "ops".to_string(),
        package_name: "ops".to_string(),
        root_module_id: "ops".to_string(),
        direct_deps: Vec::new(),
        direct_dep_ids: Vec::new(),
        package_display_names: test_package_display_names_with_deps(
            "ops".to_string(),
            "ops".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        package_direct_dep_ids: test_package_direct_dep_ids(
            "ops".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        runtime_requirements: Vec::new(),
        foreword_index: Vec::new(),
        foreword_registrations: Vec::new(),
        module_aliases: BTreeMap::new(),
        opaque_family_types: BTreeMap::new(),
        entrypoints: Vec::new(),
        native_callbacks: Vec::new(),
        shackle_decls: Vec::new(),
        binding_layouts: Vec::new(),
        owners: Vec::new(),
        routines: vec![
            RuntimeRoutinePlan {
                package_id: test_package_id_for_module("ops"),
                module_id: "ops".to_string(),
                routine_key: "ops#impl-0-method-0".to_string(),
                symbol_name: "send".to_string(),
                symbol_kind: "fn".to_string(),
                exported: false,
                is_async: false,
                type_params: vec!["T".to_string()],
                behavior_attrs: BTreeMap::new(),
                params: vec![RuntimeParamPlan {
                    binding_id: 0,
                    mode: Some("read".to_string()),
                    name: "self".to_string(),
                    ty: parse_routine_type_text("std.concurrent.Channel[T]").expect("type"),
                }],
                return_type: test_return_type(
                    "fn send(read self: std.concurrent.Channel[T]) -> Int:",
                ),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                cleanup_footers: Vec::new(),
                statements: Vec::new(),
            },
            RuntimeRoutinePlan {
                package_id: test_package_id_for_module("ops"),
                module_id: "ops".to_string(),
                routine_key: "ops#impl-1-method-0".to_string(),
                symbol_name: "send".to_string(),
                symbol_kind: "fn".to_string(),
                exported: false,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: vec![RuntimeParamPlan {
                    binding_id: 0,
                    mode: Some("read".to_string()),
                    name: "self".to_string(),
                    ty: parse_routine_type_text("std.concurrent.Channel[Bool]").expect("type"),
                }],
                return_type: test_return_type(
                    "fn send(read self: std.concurrent.Channel[Bool]) -> Int:",
                ),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                cleanup_footers: Vec::new(),
                statements: Vec::new(),
            },
        ],
    };
    let mut state = RuntimeExecutionState::default();
    let channel = insert_runtime_channel(&mut state, &["Int".to_string()], 0);

    let index = resolve_routine_index_for_call(
        &plan,
        "ops",
        "ops",
        &["ops".to_string(), "send".to_string()],
        &[RuntimeCallArg {
            name: None,
            value: RuntimeValue::Opaque(RuntimeOpaqueValue::Channel(channel)),
            source_expr: ParsedExpr::Path(vec!["chan".to_string()]),
        }],
        None,
        None,
        true,
        Some(&state),
    )
    .expect("dynamic bare method should resolve")
    .expect("call should resolve");

    assert_eq!(index, 0);
}

#[test]
fn runtime_dynamic_bare_method_fallback_matches_binding_handle_receiver() {
    let plan = RuntimePackagePlan {
        package_id: "process".to_string(),
        package_name: "process".to_string(),
        root_module_id: "process".to_string(),
        direct_deps: Vec::new(),
        direct_dep_ids: Vec::new(),
        package_display_names: test_package_display_names_with_deps(
            "process".to_string(),
            "process".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        package_direct_dep_ids: test_package_direct_dep_ids(
            "process".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        runtime_requirements: Vec::new(),
        foreword_index: Vec::new(),
        foreword_registrations: Vec::new(),
        module_aliases: BTreeMap::new(),
        opaque_family_types: BTreeMap::from([(
            "file_stream_handle".to_string(),
            vec!["arcana_winapi.process_handles.FileStream".to_string()],
        )]),
        entrypoints: Vec::new(),
        native_callbacks: Vec::new(),
        shackle_decls: Vec::new(),
        binding_layouts: Vec::new(),
        owners: Vec::new(),
        routines: vec![RuntimeRoutinePlan {
            package_id: test_package_id_for_module("process"),
            module_id: "process".to_string(),
            routine_key: "process#impl-0-method-0".to_string(),
            symbol_name: "path".to_string(),
            symbol_kind: "fn".to_string(),
            exported: false,
            is_async: false,
            type_params: Vec::new(),
            behavior_attrs: BTreeMap::new(),
            params: vec![RuntimeParamPlan {
                binding_id: 0,
                mode: Some("read".to_string()),
                name: "self".to_string(),
                ty: parse_routine_type_text("arcana_winapi.process_handles.FileStream")
                    .expect("type"),
            }],
            return_type: test_return_type(
                "fn path(read self: arcana_winapi.process_handles.FileStream) -> Str:",
            ),
            intrinsic_impl: None,
            native_impl: None,
            impl_target_type: None,
            impl_trait_path: None,
            availability: Vec::new(),
            cleanup_footers: Vec::new(),
            statements: Vec::new(),
        }],
    };
    let state = RuntimeExecutionState::default();

    let index = resolve_routine_index_for_call(
        &plan,
        "process",
        "process",
        &["process".to_string(), "path".to_string()],
        &[RuntimeCallArg {
            name: None,
            value: RuntimeValue::Opaque(RuntimeOpaqueValue::Binding(RuntimeBindingOpaqueValue {
                package_id: "arcana_winapi",
                type_name: "arcana_winapi.process_handles.FileStream",
                handle: 1,
            })),
            source_expr: ParsedExpr::Path(vec!["stream".to_string()]),
        }],
        None,
        None,
        true,
        Some(&state),
    )
    .expect("dynamic bare method should resolve")
    .expect("call should resolve");

    assert_eq!(index, 0);
}

#[test]
fn runtime_dynamic_bare_method_fallback_keeps_owner_identity() {
    fn synthetic_owner_type(owner_key: &str) -> IrRoutineType {
        let mut ty = parse_routine_type_text("Owner").expect("type");
        if let IrRoutineTypeKind::Path(path) = &mut ty.kind {
            path.segments[0] = format!("Owner<{owner_key}>");
        }
        ty
    }

    let owner_counter = synthetic_owner_type("app.Counter");
    let owner_timer = synthetic_owner_type("app.Timer");
    let plan = RuntimePackagePlan {
        package_id: "app".to_string(),
        package_name: "app".to_string(),
        root_module_id: "app".to_string(),
        direct_deps: Vec::new(),
        direct_dep_ids: Vec::new(),
        package_display_names: test_package_display_names_with_deps(
            "app".to_string(),
            "app".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        package_direct_dep_ids: test_package_direct_dep_ids(
            "app".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        runtime_requirements: Vec::new(),
        foreword_index: Vec::new(),
        foreword_registrations: Vec::new(),
        module_aliases: BTreeMap::new(),
        opaque_family_types: BTreeMap::new(),
        entrypoints: Vec::new(),
        native_callbacks: Vec::new(),
        shackle_decls: Vec::new(),
        binding_layouts: Vec::new(),
        owners: Vec::new(),
        routines: vec![
            RuntimeRoutinePlan {
                package_id: test_package_id_for_module("app"),
                module_id: "app".to_string(),
                routine_key: "app#impl-0-method-0".to_string(),
                symbol_name: "tick".to_string(),
                symbol_kind: "fn".to_string(),
                exported: false,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: vec![RuntimeParamPlan {
                    binding_id: 0,
                    mode: Some("read".to_string()),
                    name: "self".to_string(),
                    ty: owner_counter.clone(),
                }],
                return_type: Some(parse_routine_type_text("Int").expect("type")),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: Some(owner_counter),
                impl_trait_path: None,
                availability: Vec::new(),
                cleanup_footers: Vec::new(),
                statements: Vec::new(),
            },
            RuntimeRoutinePlan {
                package_id: test_package_id_for_module("app"),
                module_id: "app".to_string(),
                routine_key: "app#impl-1-method-0".to_string(),
                symbol_name: "tick".to_string(),
                symbol_kind: "fn".to_string(),
                exported: false,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: vec![RuntimeParamPlan {
                    binding_id: 0,
                    mode: Some("read".to_string()),
                    name: "self".to_string(),
                    ty: owner_timer.clone(),
                }],
                return_type: Some(parse_routine_type_text("Int").expect("type")),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: Some(owner_timer),
                impl_trait_path: None,
                availability: Vec::new(),
                cleanup_footers: Vec::new(),
                statements: Vec::new(),
            },
        ],
    };
    let state = RuntimeExecutionState::default();

    let index = resolve_routine_index_for_call(
        &plan,
        "app",
        "app",
        &["tick".to_string()],
        &[RuntimeCallArg {
            name: None,
            value: RuntimeValue::OwnerHandle("app.Counter".to_string()),
            source_expr: ParsedExpr::Path(vec!["self".to_string()]),
        }],
        None,
        None,
        true,
        Some(&state),
    )
    .expect("dynamic bare method should resolve")
    .expect("call should resolve");

    assert_eq!(index, 0);
}

#[test]
fn runtime_dynamic_bare_method_fallback_rejects_wrong_sole_candidate() {
    let self_type = parse_routine_type_text("AtomicInt").expect("type");
    let plan = RuntimePackagePlan {
        package_id: "ops".to_string(),
        package_name: "ops".to_string(),
        root_module_id: "ops".to_string(),
        direct_deps: Vec::new(),
        direct_dep_ids: Vec::new(),
        package_display_names: test_package_display_names_with_deps(
            "ops".to_string(),
            "ops".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        package_direct_dep_ids: test_package_direct_dep_ids(
            "ops".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        runtime_requirements: Vec::new(),
        foreword_index: Vec::new(),
        foreword_registrations: Vec::new(),
        module_aliases: BTreeMap::new(),
        opaque_family_types: BTreeMap::new(),
        entrypoints: Vec::new(),
        native_callbacks: Vec::new(),
        shackle_decls: Vec::new(),
        binding_layouts: Vec::new(),
        owners: Vec::new(),
        routines: vec![RuntimeRoutinePlan {
            package_id: test_package_id_for_module("ops"),
            module_id: "ops".to_string(),
            routine_key: "ops#impl-0-method-0".to_string(),
            symbol_name: "tick".to_string(),
            symbol_kind: "fn".to_string(),
            exported: false,
            is_async: false,
            type_params: Vec::new(),
            behavior_attrs: BTreeMap::new(),
            params: vec![RuntimeParamPlan {
                binding_id: 0,
                mode: Some("read".to_string()),
                name: "self".to_string(),
                ty: self_type.clone(),
            }],
            return_type: test_return_type("fn tick(read self: AtomicInt) -> Int:"),
            intrinsic_impl: None,
            native_impl: None,
            impl_target_type: Some(self_type),
            impl_trait_path: None,
            availability: Vec::new(),
            cleanup_footers: Vec::new(),
            statements: Vec::new(),
        }],
    };

    let err = resolve_routine_index_for_call(
        &plan,
        "ops",
        "ops",
        &["tick".to_string()],
        &[RuntimeCallArg {
            name: None,
            value: RuntimeValue::Bool(true),
            source_expr: ParsedExpr::Bool(true),
        }],
        None,
        None,
        true,
        None,
    )
    .expect_err("bare method fallback should validate the sole candidate receiver");

    assert!(
        err.contains("no overload matching receiver `Bool`"),
        "{err}"
    );
}

#[test]
fn runtime_dynamic_bare_method_fallback_rejects_qualified_leaf_collision() {
    let self_type = parse_routine_type_text("pkg_a.Counter").expect("type");
    let receiver = RuntimeValue::Record {
        name: "pkg_b.Counter".to_string(),
        fields: BTreeMap::new(),
    };
    let plan = RuntimePackagePlan {
        package_id: "app".to_string(),
        package_name: "app".to_string(),
        root_module_id: "app".to_string(),
        direct_deps: Vec::new(),
        direct_dep_ids: Vec::new(),
        package_display_names: test_package_display_names_with_deps(
            "app".to_string(),
            "app".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        package_direct_dep_ids: test_package_direct_dep_ids(
            "app".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        runtime_requirements: Vec::new(),
        foreword_index: Vec::new(),
        foreword_registrations: Vec::new(),
        module_aliases: BTreeMap::new(),
        opaque_family_types: BTreeMap::new(),
        entrypoints: Vec::new(),
        native_callbacks: Vec::new(),
        shackle_decls: Vec::new(),
        binding_layouts: Vec::new(),
        owners: Vec::new(),
        routines: vec![RuntimeRoutinePlan {
            package_id: test_package_id_for_module("app"),
            module_id: "app".to_string(),
            routine_key: "app#impl-0-method-0".to_string(),
            symbol_name: "tick".to_string(),
            symbol_kind: "fn".to_string(),
            exported: false,
            is_async: false,
            type_params: Vec::new(),
            behavior_attrs: BTreeMap::new(),
            params: vec![RuntimeParamPlan {
                binding_id: 0,
                mode: Some("read".to_string()),
                name: "self".to_string(),
                ty: self_type.clone(),
            }],
            return_type: test_return_type("fn tick(read self: pkg_a.Counter) -> Int:"),
            intrinsic_impl: None,
            native_impl: None,
            impl_target_type: Some(self_type),
            impl_trait_path: None,
            availability: Vec::new(),
            cleanup_footers: Vec::new(),
            statements: Vec::new(),
        }],
    };

    let err = resolve_routine_index_for_call(
        &plan,
        "app",
        "app",
        &["tick".to_string()],
        &[RuntimeCallArg {
            name: None,
            value: receiver,
            source_expr: ParsedExpr::Path(vec!["counter".to_string()]),
        }],
        None,
        None,
        true,
        None,
    )
    .expect_err("qualified receiver collisions should not match by leaf name");

    assert!(
        err.contains("no overload matching receiver `Counter`"),
        "{err}"
    );
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
    assert_eq!(code, 0, "stdout={:?}", host.stdout);
    assert_eq!(host.stdout, vec!["hello, arcana".to_string()]);
}

#[test]
fn runtime_json_abi_manifest_lists_exported_callable_routines() {
    let plan = plan_from_artifact(&sample_return_artifact()).expect("runtime plan should build");
    let manifest = render_exported_json_abi_manifest(&plan).expect("json abi manifest");
    let value = manifest
        .parse::<serde_json::Value>()
        .expect("manifest should parse as json");
    assert_eq!(value["format"].as_str(), Some("arcana-runtime-json-abi-v4"));
    let routines = value["routines"]
        .as_array()
        .expect("manifest routines should be an array");
    assert_eq!(routines.len(), 1);
    assert_eq!(routines[0]["routine_key"].as_str(), Some("hello#sym-0"));
    assert_eq!(routines[0]["params"], serde_json::json!([]));
    assert_eq!(routines[0]["return_type"].as_str(), Some("Int"));
}

#[test]
fn runtime_json_abi_manifest_records_binding_raw_metadata() {
    let mut plan = empty_runtime_plan("hostapi");
    plan.routines = vec![RuntimeRoutinePlan {
        package_id: "hostapi".to_string(),
        module_id: "hostapi.raw".to_string(),
        routine_key: "hostapi#fn-0".to_string(),
        symbol_name: "BumpRect".to_string(),
        symbol_kind: "fn".to_string(),
        exported: true,
        is_async: false,
        type_params: Vec::new(),
        behavior_attrs: BTreeMap::new(),
        params: vec![RuntimeParamPlan {
            binding_id: 0,
            mode: Some("edit".to_string()),
            name: "rect".to_string(),
            ty: parse_routine_type_text("hostapi.raw.Rect").expect("type should parse"),
        }],
        return_type: test_return_type("fn BumpRect() -> hostapi.raw.Mode:"),
        intrinsic_impl: None,
        native_impl: Some("hostapi.raw.BumpRect".to_string()),
        impl_target_type: None,
        impl_trait_path: None,
        availability: Vec::new(),
        cleanup_footers: Vec::new(),
        statements: Vec::new(),
    }];
    plan.native_callbacks = vec![RuntimeNativeCallbackPlan {
        package_id: "hostapi".to_string(),
        module_id: "hostapi.raw".to_string(),
        name: "hostapi.raw.WindowProc".to_string(),
        params: vec![RuntimeParamPlan {
            binding_id: 0,
            mode: None,
            name: "message".to_string(),
            ty: parse_routine_type_text("I32").expect("type should parse"),
        }],
        return_type: test_return_type("fn WindowProc() -> I32:"),
        target: vec![
            "hostapi".to_string(),
            "callbacks".to_string(),
            "proc".to_string(),
        ],
        target_routine_key: None,
    }];
    plan.binding_layouts = vec![
        arcana_cabi::ArcanaCabiBindingLayout {
            layout_id: "hostapi.raw.Rect".to_string(),
            size: 12,
            align: 4,
            kind: arcana_cabi::ArcanaCabiBindingLayoutKind::Struct {
                fields: vec![
                    arcana_cabi::ArcanaCabiBindingLayoutField {
                        name: "left".to_string(),
                        ty: arcana_cabi::ArcanaCabiBindingRawType::Scalar(
                            arcana_cabi::ArcanaCabiBindingScalarType::I32,
                        ),
                        offset: 0,
                        bit_width: None,
                        bit_offset: None,
                    },
                    arcana_cabi::ArcanaCabiBindingLayoutField {
                        name: "top".to_string(),
                        ty: arcana_cabi::ArcanaCabiBindingRawType::Scalar(
                            arcana_cabi::ArcanaCabiBindingScalarType::I32,
                        ),
                        offset: 4,
                        bit_width: None,
                        bit_offset: None,
                    },
                    arcana_cabi::ArcanaCabiBindingLayoutField {
                        name: "flags".to_string(),
                        ty: arcana_cabi::ArcanaCabiBindingRawType::Scalar(
                            arcana_cabi::ArcanaCabiBindingScalarType::U32,
                        ),
                        offset: 8,
                        bit_width: Some(3),
                        bit_offset: Some(0),
                    },
                ],
            },
        },
        arcana_cabi::ArcanaCabiBindingLayout {
            layout_id: "hostapi.raw.Mode".to_string(),
            size: 4,
            align: 4,
            kind: arcana_cabi::ArcanaCabiBindingLayoutKind::Enum {
                repr: arcana_cabi::ArcanaCabiBindingScalarType::U32,
                variants: vec![
                    arcana_cabi::ArcanaCabiBindingLayoutEnumVariant {
                        name: "Idle".to_string(),
                        value: 0,
                    },
                    arcana_cabi::ArcanaCabiBindingLayoutEnumVariant {
                        name: "Busy".to_string(),
                        value: 1,
                    },
                ],
            },
        },
    ];

    let manifest = render_exported_json_abi_manifest(&plan).expect("json abi manifest");
    let value = manifest
        .parse::<serde_json::Value>()
        .expect("manifest should parse as json");
    let binding = value["binding"]
        .as_object()
        .expect("binding json abi section should exist");
    assert_eq!(
        binding["imports"][0]["name"].as_str(),
        Some("hostapi.raw.BumpRect")
    );
    assert_eq!(
        binding["imports"][0]["params"][0]["input_type"].as_str(),
        Some("hostapi.raw.Rect")
    );
    assert_eq!(
        binding["imports"][0]["params"][0]["write_back_type"].as_str(),
        Some("hostapi.raw.Rect")
    );
    assert_eq!(
        binding["callbacks"][0]["name"].as_str(),
        Some("hostapi.raw.WindowProc")
    );
    assert_eq!(
        binding["layouts"][1]["layout_id"].as_str(),
        Some("hostapi.raw.Mode")
    );
}

#[test]
fn runtime_json_abi_executes_exported_routine() {
    let plan = RuntimePackagePlan {
        package_id: "tool".to_string(),
        package_name: "tool".to_string(),
        root_module_id: "tool".to_string(),
        direct_deps: Vec::new(),
        direct_dep_ids: Vec::new(),
        package_display_names: test_package_display_names_with_deps(
            "tool".to_string(),
            "tool".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        package_direct_dep_ids: test_package_direct_dep_ids(
            "tool".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        runtime_requirements: Vec::new(),
        foreword_index: Vec::new(),
        foreword_registrations: Vec::new(),
        module_aliases: BTreeMap::new(),
        opaque_family_types: BTreeMap::new(),
        entrypoints: Vec::new(),
        native_callbacks: Vec::new(),
        shackle_decls: Vec::new(),
        binding_layouts: Vec::new(),
        owners: Vec::new(),
        routines: vec![RuntimeRoutinePlan {
            package_id: test_package_id_for_module("tool"),
            module_id: "tool".to_string(),
            routine_key: "tool#fn-0".to_string(),
            symbol_name: "answer".to_string(),
            symbol_kind: "fn".to_string(),
            exported: true,
            is_async: false,
            type_params: Vec::new(),
            behavior_attrs: BTreeMap::new(),
            params: vec![RuntimeParamPlan {
                binding_id: 0,
                mode: None,
                name: "value".to_string(),
                ty: parse_routine_type_text("Int").expect("type"),
            }],
            return_type: test_return_type("fn answer(value: Int) -> Int:"),
            intrinsic_impl: None,
            native_impl: None,
            impl_target_type: None,
            impl_trait_path: None,
            availability: Vec::new(),
            cleanup_footers: Vec::new(),
            statements: vec![ParsedStmt::ReturnValue {
                value: ParsedExpr::Binary {
                    op: arcana_ir::ExecBinaryOp::Add,
                    left: Box::new(ParsedExpr::Path(vec!["value".to_string()])),
                    right: Box::new(ParsedExpr::Int(2)),
                },
            }],
        }],
    };
    let mut host = BufferedHost::default();
    let result = execute_exported_json_abi_routine(&plan, "tool#fn-0", "[5]", &mut host)
        .expect("json abi invoke should succeed");
    let result = result
        .parse::<serde_json::Value>()
        .expect("json abi result should parse");
    assert_eq!(result["result"], serde_json::Value::from(7));
    assert_eq!(result["write_backs"], serde_json::json!([]));
}

#[test]
fn runtime_json_abi_round_trips_owned_buffer_payloads() {
    let plan = RuntimePackagePlan {
        package_id: "tool".to_string(),
        package_name: "tool".to_string(),
        root_module_id: "tool".to_string(),
        direct_deps: Vec::new(),
        direct_dep_ids: Vec::new(),
        package_display_names: test_package_display_names_with_deps(
            "tool".to_string(),
            "tool".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        package_direct_dep_ids: test_package_direct_dep_ids(
            "tool".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        runtime_requirements: Vec::new(),
        foreword_index: Vec::new(),
        foreword_registrations: Vec::new(),
        module_aliases: BTreeMap::new(),
        opaque_family_types: BTreeMap::new(),
        entrypoints: Vec::new(),
        native_callbacks: Vec::new(),
        shackle_decls: Vec::new(),
        binding_layouts: Vec::new(),
        owners: Vec::new(),
        routines: vec![
            RuntimeRoutinePlan {
                package_id: test_package_id_for_module("tool"),
                module_id: "tool".to_string(),
                routine_key: "tool#fn-0".to_string(),
                symbol_name: "mirror_bytes".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: vec![RuntimeParamPlan {
                    binding_id: 0,
                    mode: None,
                    name: "buf".to_string(),
                    ty: parse_routine_type_text("ByteBuffer").expect("type"),
                }],
                return_type: test_return_type(
                    "fn mirror_bytes(read buf: ByteBuffer) -> ByteBuffer:",
                ),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                cleanup_footers: Vec::new(),
                statements: vec![ParsedStmt::ReturnValue {
                    value: ParsedExpr::Path(vec!["buf".to_string()]),
                }],
            },
            RuntimeRoutinePlan {
                package_id: test_package_id_for_module("tool"),
                module_id: "tool".to_string(),
                routine_key: "tool#fn-1".to_string(),
                symbol_name: "mirror_utf16".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: vec![RuntimeParamPlan {
                    binding_id: 0,
                    mode: None,
                    name: "buf".to_string(),
                    ty: parse_routine_type_text("Utf16Buffer").expect("type"),
                }],
                return_type: test_return_type(
                    "fn mirror_utf16(read buf: Utf16Buffer) -> Utf16Buffer:",
                ),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                cleanup_footers: Vec::new(),
                statements: vec![ParsedStmt::ReturnValue {
                    value: ParsedExpr::Path(vec!["buf".to_string()]),
                }],
            },
        ],
    };
    let mut host = BufferedHost::default();

    let bytes = execute_exported_json_abi_routine(
        &plan,
        "tool#fn-0",
        r#"[{"$byte_buffer":[65,66,67]}]"#,
        &mut host,
    )
    .expect("json abi byte buffer invoke should succeed");
    let bytes = bytes
        .parse::<serde_json::Value>()
        .expect("json abi byte buffer result should parse");
    assert_eq!(
        bytes["result"],
        serde_json::json!({ "$byte_buffer": [65, 66, 67] })
    );
    assert_eq!(bytes["write_backs"], serde_json::json!([]));

    let utf16 = execute_exported_json_abi_routine(
        &plan,
        "tool#fn-1",
        r#"[{"$utf16_buffer":[65,66,9731]}]"#,
        &mut host,
    )
    .expect("json abi utf16 buffer invoke should succeed");
    let utf16 = utf16
        .parse::<serde_json::Value>()
        .expect("json abi utf16 buffer result should parse");
    assert_eq!(
        utf16["result"],
        serde_json::json!({ "$utf16_buffer": [65, 66, 9731] })
    );
    assert_eq!(utf16["write_backs"], serde_json::json!([]));
}

#[test]
fn runtime_json_abi_manifest_records_cabi_param_metadata() {
    let plan = RuntimePackagePlan {
        package_id: "tool".to_string(),
        package_name: "tool".to_string(),
        root_module_id: "tool".to_string(),
        direct_deps: Vec::new(),
        direct_dep_ids: Vec::new(),
        package_display_names: test_package_display_names_with_deps(
            "tool".to_string(),
            "tool".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        package_direct_dep_ids: test_package_direct_dep_ids(
            "tool".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        runtime_requirements: Vec::new(),
        foreword_index: Vec::new(),
        foreword_registrations: Vec::new(),
        module_aliases: BTreeMap::new(),
        opaque_family_types: BTreeMap::new(),
        entrypoints: Vec::new(),
        native_callbacks: Vec::new(),
        shackle_decls: Vec::new(),
        binding_layouts: Vec::new(),
        owners: Vec::new(),
        routines: vec![RuntimeRoutinePlan {
            package_id: test_package_id_for_module("tool"),
            module_id: "tool".to_string(),
            routine_key: "tool#fn-0".to_string(),
            symbol_name: "bump".to_string(),
            symbol_kind: "fn".to_string(),
            exported: true,
            is_async: false,
            type_params: Vec::new(),
            behavior_attrs: BTreeMap::new(),
            params: vec![RuntimeParamPlan {
                binding_id: 0,
                mode: Some("edit".to_string()),
                name: "value".to_string(),
                ty: parse_routine_type_text("Int").expect("type"),
            }],
            return_type: test_return_type("fn bump(edit value: Int) -> Int:"),
            intrinsic_impl: None,
            native_impl: None,
            impl_target_type: None,
            impl_trait_path: None,
            availability: Vec::new(),
            cleanup_footers: Vec::new(),
            statements: vec![ParsedStmt::ReturnValue {
                value: ParsedExpr::Path(vec!["value".to_string()]),
            }],
        }],
    };
    let manifest = render_exported_json_abi_manifest(&plan).expect("json abi manifest");
    let value = manifest
        .parse::<serde_json::Value>()
        .expect("manifest should parse as json");
    let params = value["routines"][0]["params"]
        .as_array()
        .expect("manifest params should be an array");
    assert_eq!(params.len(), 1);
    assert_eq!(params[0]["name"], serde_json::Value::from("value"));
    assert_eq!(params[0]["source_mode"], serde_json::Value::from("edit"));
    assert_eq!(params[0]["input_type"], serde_json::Value::from("Int"));
    assert_eq!(
        params[0]["pass_mode"],
        serde_json::Value::from("in_with_write_back")
    );
    assert_eq!(params[0]["write_back_type"], serde_json::Value::from("Int"));
}

#[test]
fn runtime_json_abi_manifest_records_in_place_buffer_edit_metadata() {
    let plan = RuntimePackagePlan {
        package_id: "tool".to_string(),
        package_name: "tool".to_string(),
        root_module_id: "tool".to_string(),
        direct_deps: Vec::new(),
        direct_dep_ids: Vec::new(),
        package_display_names: test_package_display_names_with_deps(
            "tool".to_string(),
            "tool".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        package_direct_dep_ids: test_package_direct_dep_ids(
            "tool".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        runtime_requirements: Vec::new(),
        foreword_index: Vec::new(),
        foreword_registrations: Vec::new(),
        module_aliases: BTreeMap::new(),
        opaque_family_types: BTreeMap::new(),
        entrypoints: Vec::new(),
        native_callbacks: Vec::new(),
        shackle_decls: Vec::new(),
        binding_layouts: Vec::new(),
        owners: Vec::new(),
        routines: vec![RuntimeRoutinePlan {
            package_id: test_package_id_for_module("tool"),
            module_id: "tool".to_string(),
            routine_key: "tool#fn-0".to_string(),
            symbol_name: "touch".to_string(),
            symbol_kind: "fn".to_string(),
            exported: true,
            is_async: false,
            type_params: Vec::new(),
            behavior_attrs: BTreeMap::new(),
            params: vec![RuntimeParamPlan {
                binding_id: 0,
                mode: Some("edit".to_string()),
                name: "bytes".to_string(),
                ty: parse_routine_type_text("ByteBuffer").expect("type"),
            }],
            return_type: test_return_type("fn touch(edit bytes: ByteBuffer) -> Int:"),
            intrinsic_impl: None,
            native_impl: None,
            impl_target_type: None,
            impl_trait_path: None,
            availability: Vec::new(),
            cleanup_footers: Vec::new(),
            statements: vec![ParsedStmt::ReturnValue {
                value: ParsedExpr::Int(0),
            }],
        }],
    };
    let manifest = render_exported_json_abi_manifest(&plan).expect("json abi manifest");
    let value = manifest
        .parse::<serde_json::Value>()
        .expect("manifest should parse as json");
    let params = value["routines"][0]["params"]
        .as_array()
        .expect("manifest params should be an array");
    assert_eq!(params.len(), 1);
    assert_eq!(params[0]["source_mode"], serde_json::Value::from("edit"));
    assert_eq!(params[0]["pass_mode"], serde_json::Value::from("in"));
    assert_eq!(
        params[0]["input_type"],
        serde_json::Value::from("ByteBuffer")
    );
    assert!(params[0]["write_back_type"].is_null());
}

#[test]
fn runtime_json_abi_manifest_projects_default_read_source_mode() {
    let plan = RuntimePackagePlan {
        package_id: "tool".to_string(),
        package_name: "tool".to_string(),
        root_module_id: "tool".to_string(),
        direct_deps: Vec::new(),
        direct_dep_ids: Vec::new(),
        package_display_names: test_package_display_names_with_deps(
            "tool".to_string(),
            "tool".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        package_direct_dep_ids: test_package_direct_dep_ids(
            "tool".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        runtime_requirements: Vec::new(),
        foreword_index: Vec::new(),
        foreword_registrations: Vec::new(),
        module_aliases: BTreeMap::new(),
        opaque_family_types: BTreeMap::new(),
        entrypoints: Vec::new(),
        native_callbacks: Vec::new(),
        shackle_decls: Vec::new(),
        binding_layouts: Vec::new(),
        owners: Vec::new(),
        routines: vec![RuntimeRoutinePlan {
            package_id: test_package_id_for_module("tool"),
            module_id: "tool".to_string(),
            routine_key: "tool#fn-0".to_string(),
            symbol_name: "answer".to_string(),
            symbol_kind: "fn".to_string(),
            exported: true,
            is_async: false,
            type_params: Vec::new(),
            behavior_attrs: BTreeMap::new(),
            params: vec![RuntimeParamPlan {
                binding_id: 0,
                mode: None,
                name: "value".to_string(),
                ty: parse_routine_type_text("Int").expect("type"),
            }],
            return_type: test_return_type("fn answer(value: Int) -> Int:"),
            intrinsic_impl: None,
            native_impl: None,
            impl_target_type: None,
            impl_trait_path: None,
            availability: Vec::new(),
            cleanup_footers: Vec::new(),
            statements: vec![ParsedStmt::ReturnValue {
                value: ParsedExpr::Path(vec!["value".to_string()]),
            }],
        }],
    };
    let manifest = render_exported_json_abi_manifest(&plan).expect("json abi manifest");
    let value = manifest
        .parse::<serde_json::Value>()
        .expect("manifest should parse as json");
    let params = value["routines"][0]["params"]
        .as_array()
        .expect("manifest params should be an array");
    assert_eq!(params.len(), 1);
    assert_eq!(params[0]["source_mode"], serde_json::Value::from("read"));
    assert_eq!(params[0]["pass_mode"], serde_json::Value::from("in"));
    assert_eq!(params[0]["input_type"], serde_json::Value::from("Int"));
    assert!(params[0]["write_back_type"].is_null());
}

#[test]
fn runtime_json_abi_writes_back_edit_arguments() {
    let plan = RuntimePackagePlan {
        package_id: "tool".to_string(),
        package_name: "tool".to_string(),
        root_module_id: "tool".to_string(),
        direct_deps: Vec::new(),
        direct_dep_ids: Vec::new(),
        package_display_names: test_package_display_names_with_deps(
            "tool".to_string(),
            "tool".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        package_direct_dep_ids: test_package_direct_dep_ids(
            "tool".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        runtime_requirements: Vec::new(),
        foreword_index: Vec::new(),
        foreword_registrations: Vec::new(),
        module_aliases: BTreeMap::new(),
        opaque_family_types: BTreeMap::new(),
        entrypoints: Vec::new(),
        native_callbacks: Vec::new(),
        shackle_decls: Vec::new(),
        binding_layouts: Vec::new(),
        owners: Vec::new(),
        routines: vec![RuntimeRoutinePlan {
            package_id: test_package_id_for_module("tool"),
            module_id: "tool".to_string(),
            routine_key: "tool#fn-0".to_string(),
            symbol_name: "bump".to_string(),
            symbol_kind: "fn".to_string(),
            exported: true,
            is_async: false,
            type_params: Vec::new(),
            behavior_attrs: BTreeMap::new(),
            params: vec![RuntimeParamPlan {
                binding_id: 0,
                mode: Some("edit".to_string()),
                name: "value".to_string(),
                ty: parse_routine_type_text("Int").expect("type"),
            }],
            return_type: test_return_type("fn bump(edit value: Int) -> Int:"),
            intrinsic_impl: None,
            native_impl: None,
            impl_target_type: None,
            impl_trait_path: None,
            availability: Vec::new(),
            cleanup_footers: Vec::new(),
            statements: vec![
                ParsedStmt::Assign {
                    target: ParsedAssignTarget::Name("value".to_string()),
                    op: ParsedAssignOp::AddAssign,
                    value: ParsedExpr::Int(2),
                },
                ParsedStmt::ReturnValue {
                    value: ParsedExpr::Path(vec!["value".to_string()]),
                },
            ],
        }],
    };
    let mut host = BufferedHost::default();
    let result = execute_exported_json_abi_routine(&plan, "tool#fn-0", "[5]", &mut host)
        .expect("json abi invoke should succeed");
    let result = result
        .parse::<serde_json::Value>()
        .expect("json abi result should parse");
    assert_eq!(result["result"], serde_json::Value::from(7));
    assert_eq!(
        result["write_backs"],
        serde_json::json!([{ "index": 0, "name": "value", "value": 7 }])
    );
}

#[test]
fn runtime_json_abi_manifest_omits_unsupported_owner_reference_and_opaque_routines() {
    let plan = RuntimePackagePlan {
        package_id: "tool".to_string(),
        package_name: "tool".to_string(),
        root_module_id: "tool".to_string(),
        direct_deps: Vec::new(),
        direct_dep_ids: Vec::new(),
        package_display_names: test_package_display_names_with_deps(
            "tool".to_string(),
            "tool".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        package_direct_dep_ids: test_package_direct_dep_ids(
            "tool".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        runtime_requirements: Vec::new(),
        foreword_index: Vec::new(),
        foreword_registrations: Vec::new(),
        module_aliases: BTreeMap::new(),
        opaque_family_types: BTreeMap::from([(
            "file_stream_handle".to_string(),
            vec!["arcana_winapi.process_handles.FileStream".to_string()],
        )]),
        entrypoints: Vec::new(),
        native_callbacks: Vec::new(),
        shackle_decls: Vec::new(),
        binding_layouts: Vec::new(),
        owners: Vec::new(),
        routines: vec![
            RuntimeRoutinePlan {
                package_id: test_package_id_for_module("tool"),
                module_id: "tool".to_string(),
                routine_key: "tool#fn-0".to_string(),
                symbol_name: "answer".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: vec![RuntimeParamPlan {
                    binding_id: 0,
                    mode: None,
                    name: "value".to_string(),
                    ty: parse_routine_type_text("Int").expect("type"),
                }],
                return_type: test_return_type("fn answer(value: Int) -> Int:"),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                cleanup_footers: Vec::new(),
                statements: vec![ParsedStmt::ReturnValue {
                    value: ParsedExpr::Path(vec!["value".to_string()]),
                }],
            },
            RuntimeRoutinePlan {
                package_id: test_package_id_for_module("tool"),
                module_id: "tool".to_string(),
                routine_key: "tool#fn-1".to_string(),
                symbol_name: "stream_path".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: vec![RuntimeParamPlan {
                    binding_id: 0,
                    mode: Some("read".to_string()),
                    name: "stream".to_string(),
                    ty: parse_routine_type_text("arcana_winapi.process_handles.FileStream")
                        .expect("type"),
                }],
                return_type: test_return_type(
                    "fn stream_path(read stream: arcana_winapi.process_handles.FileStream) -> Int:",
                ),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                cleanup_footers: Vec::new(),
                statements: Vec::new(),
            },
            RuntimeRoutinePlan {
                package_id: test_package_id_for_module("tool"),
                module_id: "tool".to_string(),
                routine_key: "tool#fn-2".to_string(),
                symbol_name: "stream_name".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: vec![RuntimeParamPlan {
                    binding_id: 0,
                    mode: Some("read".to_string()),
                    name: "stream".to_string(),
                    ty: parse_routine_type_text("arcana_winapi.process_handles.FileStream")
                        .expect("type"),
                }],
                return_type: test_return_type(
                    "fn stream_name(read stream: arcana_winapi.process_handles.FileStream) -> Int:",
                ),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                cleanup_footers: Vec::new(),
                statements: Vec::new(),
            },
            RuntimeRoutinePlan {
                package_id: test_package_id_for_module("tool"),
                module_id: "tool".to_string(),
                routine_key: "tool#fn-3".to_string(),
                symbol_name: "owner_only".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: vec![RuntimeParamPlan {
                    binding_id: 0,
                    mode: Some("read".to_string()),
                    name: "owner".to_string(),
                    ty: parse_routine_type_text("Owner").expect("type"),
                }],
                return_type: test_return_type("fn owner_only(read owner: Owner) -> Int:"),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                cleanup_footers: Vec::new(),
                statements: Vec::new(),
            },
        ],
    };

    let manifest = render_exported_json_abi_manifest(&plan).expect("json abi manifest");
    let value = manifest
        .parse::<serde_json::Value>()
        .expect("manifest should parse as json");
    let routines = value["routines"]
        .as_array()
        .expect("manifest routines should be an array");

    assert_eq!(routines.len(), 1);
    assert_eq!(routines[0]["routine_key"].as_str(), Some("tool#fn-0"));
}

#[test]
fn runtime_json_abi_rejects_executing_unsupported_exported_routine() {
    let plan = RuntimePackagePlan {
        package_id: "tool".to_string(),
        package_name: "tool".to_string(),
        root_module_id: "tool".to_string(),
        direct_deps: Vec::new(),
        direct_dep_ids: Vec::new(),
        package_display_names: test_package_display_names_with_deps(
            "tool".to_string(),
            "tool".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        package_direct_dep_ids: test_package_direct_dep_ids(
            "tool".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        runtime_requirements: Vec::new(),
        foreword_index: Vec::new(),
        foreword_registrations: Vec::new(),
        module_aliases: BTreeMap::new(),
        opaque_family_types: BTreeMap::new(),
        entrypoints: Vec::new(),
        native_callbacks: Vec::new(),
        shackle_decls: Vec::new(),
        binding_layouts: Vec::new(),
        owners: Vec::new(),
        routines: vec![RuntimeRoutinePlan {
            package_id: test_package_id_for_module("tool"),
            module_id: "tool".to_string(),
            routine_key: "tool#fn-0".to_string(),
            symbol_name: "stream_path".to_string(),
            symbol_kind: "fn".to_string(),
            exported: true,
            is_async: false,
            type_params: Vec::new(),
            behavior_attrs: BTreeMap::new(),
            params: vec![RuntimeParamPlan {
                binding_id: 0,
                mode: Some("read".to_string()),
                name: "stream".to_string(),
                ty: parse_routine_type_text("arcana_winapi.process_handles.FileStream")
                    .expect("type"),
            }],
            return_type: test_return_type(
                "fn stream_path(read stream: arcana_winapi.process_handles.FileStream) -> Int:",
            ),
            intrinsic_impl: None,
            native_impl: None,
            impl_target_type: None,
            impl_trait_path: None,
            availability: Vec::new(),
            cleanup_footers: Vec::new(),
            statements: Vec::new(),
        }],
    };
    let mut host = BufferedHost::default();

    let err = execute_exported_json_abi_routine(&plan, "tool#fn-0", "[5]", &mut host)
        .expect_err("json abi should reject unsupported exported routine signatures");

    assert!(err.contains("not exported or callable"), "{err}");
}

#[test]
fn runtime_native_abi_executes_exported_routine() {
    let plan = RuntimePackagePlan {
        package_id: "tool".to_string(),
        package_name: "tool".to_string(),
        root_module_id: "tool".to_string(),
        direct_deps: Vec::new(),
        direct_dep_ids: Vec::new(),
        package_display_names: test_package_display_names_with_deps(
            "tool".to_string(),
            "tool".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        package_direct_dep_ids: test_package_direct_dep_ids(
            "tool".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        runtime_requirements: Vec::new(),
        foreword_index: Vec::new(),
        foreword_registrations: Vec::new(),
        module_aliases: BTreeMap::new(),
        opaque_family_types: BTreeMap::new(),
        entrypoints: Vec::new(),
        native_callbacks: Vec::new(),
        shackle_decls: Vec::new(),
        binding_layouts: Vec::new(),
        owners: Vec::new(),
        routines: vec![RuntimeRoutinePlan {
            package_id: test_package_id_for_module("tool"),
            module_id: "tool".to_string(),
            routine_key: "tool#fn-0".to_string(),
            symbol_name: "answer".to_string(),
            symbol_kind: "fn".to_string(),
            exported: true,
            is_async: false,
            type_params: Vec::new(),
            behavior_attrs: BTreeMap::new(),
            params: vec![RuntimeParamPlan {
                binding_id: 0,
                mode: None,
                name: "value".to_string(),
                ty: parse_routine_type_text("Int").expect("type"),
            }],
            return_type: test_return_type("fn answer(value: Int) -> Int:"),
            intrinsic_impl: None,
            native_impl: None,
            impl_target_type: None,
            impl_trait_path: None,
            availability: Vec::new(),
            cleanup_footers: Vec::new(),
            statements: vec![ParsedStmt::ReturnValue {
                value: ParsedExpr::Binary {
                    op: arcana_ir::ExecBinaryOp::Add,
                    left: Box::new(ParsedExpr::Path(vec!["value".to_string()])),
                    right: Box::new(ParsedExpr::Int(2)),
                },
            }],
        }],
    };
    let mut host = BufferedHost::default();
    let result = execute_exported_abi_routine(
        &plan,
        "tool#fn-0",
        vec![super::RuntimeAbiValue::Int(5)],
        &mut host,
    )
    .expect("native abi invoke should succeed");
    assert_eq!(result.result, super::RuntimeAbiValue::Int(7));
    assert!(result.write_backs.is_empty());
}

#[test]
fn runtime_native_abi_supports_string_and_byte_values() {
    let plan = RuntimePackagePlan {
        package_id: "tool".to_string(),
        package_name: "tool".to_string(),
        root_module_id: "tool".to_string(),
        direct_deps: Vec::new(),
        direct_dep_ids: Vec::new(),
        package_display_names: test_package_display_names_with_deps(
            "tool".to_string(),
            "tool".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        package_direct_dep_ids: test_package_direct_dep_ids(
            "tool".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        runtime_requirements: Vec::new(),
        foreword_index: Vec::new(),
        foreword_registrations: Vec::new(),
        module_aliases: BTreeMap::new(),
        opaque_family_types: BTreeMap::new(),
        entrypoints: Vec::new(),
        native_callbacks: Vec::new(),
        shackle_decls: Vec::new(),
        binding_layouts: Vec::new(),
        owners: Vec::new(),
        routines: vec![
            RuntimeRoutinePlan {
                package_id: test_package_id_for_module("tool"),
                module_id: "tool".to_string(),
                routine_key: "tool#fn-0".to_string(),
                symbol_name: "greet".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: vec![RuntimeParamPlan {
                    binding_id: 0,
                    mode: Some("read".to_string()),
                    name: "name".to_string(),
                    ty: parse_routine_type_text("Str").expect("type"),
                }],
                return_type: test_return_type("fn greet(read name: Str) -> Str:"),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                cleanup_footers: Vec::new(),
                statements: vec![ParsedStmt::ReturnValue {
                    value: ParsedExpr::Binary {
                        op: arcana_ir::ExecBinaryOp::Add,
                        left: Box::new(ParsedExpr::Str("hi ".to_string())),
                        right: Box::new(ParsedExpr::Path(vec!["name".to_string()])),
                    },
                }],
            },
            RuntimeRoutinePlan {
                package_id: test_package_id_for_module("tool"),
                module_id: "tool".to_string(),
                routine_key: "tool#fn-1".to_string(),
                symbol_name: "tail".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: vec![RuntimeParamPlan {
                    binding_id: 0,
                    mode: Some("read".to_string()),
                    name: "bytes".to_string(),
                    ty: parse_routine_type_text("Bytes").expect("type"),
                }],
                return_type: test_return_type("fn tail(read bytes: Bytes) -> Bytes:"),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                cleanup_footers: Vec::new(),
                statements: vec![ParsedStmt::ReturnValue {
                    value: ParsedExpr::Slice {
                        expr: Box::new(ParsedExpr::Path(vec!["bytes".to_string()])),
                        family: ParsedProjectionFamily::Inferred,
                        start: Some(Box::new(ParsedExpr::Int(1))),
                        end: None,
                        len: None,
                        stride: None,
                        inclusive_end: false,
                    },
                }],
            },
            RuntimeRoutinePlan {
                package_id: test_package_id_for_module("tool"),
                module_id: "tool".to_string(),
                routine_key: "tool#fn-2".to_string(),
                symbol_name: "echo_pair".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: vec![RuntimeParamPlan {
                    binding_id: 0,
                    mode: Some("read".to_string()),
                    name: "pair".to_string(),
                    ty: parse_routine_type_text("Pair[Str, Int]").expect("type"),
                }],
                return_type: test_return_type(
                    "fn echo_pair(read pair: Pair[Str, Int]) -> Pair[Str, Int]:",
                ),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                cleanup_footers: Vec::new(),
                statements: vec![ParsedStmt::ReturnValue {
                    value: ParsedExpr::Path(vec!["pair".to_string()]),
                }],
            },
        ],
    };
    let mut host = BufferedHost::default();
    let greet = execute_exported_abi_routine(
        &plan,
        "tool#fn-0",
        vec![super::RuntimeAbiValue::Str("arcana".to_string())],
        &mut host,
    )
    .expect("string abi invoke should succeed");
    assert_eq!(
        greet.result,
        super::RuntimeAbiValue::Str("hi arcana".to_string())
    );
    assert!(greet.write_backs.is_empty());

    let tail = execute_exported_abi_routine(
        &plan,
        "tool#fn-1",
        vec![super::RuntimeAbiValue::Bytes(b"arc".to_vec())],
        &mut host,
    )
    .expect("byte abi invoke should succeed");
    assert_eq!(tail.result, super::RuntimeAbiValue::Bytes(b"rc".to_vec()));
    assert!(tail.write_backs.is_empty());

    let echoed = execute_exported_abi_routine(
        &plan,
        "tool#fn-2",
        vec![super::RuntimeAbiValue::Pair(
            Box::new(super::RuntimeAbiValue::Str("arcana".to_string())),
            Box::new(super::RuntimeAbiValue::Int(7)),
        )],
        &mut host,
    )
    .expect("pair abi invoke should succeed");
    assert_eq!(
        echoed.result,
        super::RuntimeAbiValue::Pair(
            Box::new(super::RuntimeAbiValue::Str("arcana".to_string())),
            Box::new(super::RuntimeAbiValue::Int(7)),
        )
    );
    assert!(echoed.write_backs.is_empty());
}

#[test]
fn runtime_native_abi_writes_back_edit_arguments() {
    let plan = RuntimePackagePlan {
        package_id: "tool".to_string(),
        package_name: "tool".to_string(),
        root_module_id: "tool".to_string(),
        direct_deps: Vec::new(),
        direct_dep_ids: Vec::new(),
        package_display_names: test_package_display_names_with_deps(
            "tool".to_string(),
            "tool".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        package_direct_dep_ids: test_package_direct_dep_ids(
            "tool".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        runtime_requirements: Vec::new(),
        foreword_index: Vec::new(),
        foreword_registrations: Vec::new(),
        module_aliases: BTreeMap::new(),
        opaque_family_types: BTreeMap::new(),
        entrypoints: Vec::new(),
        native_callbacks: Vec::new(),
        shackle_decls: Vec::new(),
        binding_layouts: Vec::new(),
        owners: Vec::new(),
        routines: vec![RuntimeRoutinePlan {
            package_id: test_package_id_for_module("tool"),
            module_id: "tool".to_string(),
            routine_key: "tool#fn-0".to_string(),
            symbol_name: "bump".to_string(),
            symbol_kind: "fn".to_string(),
            exported: true,
            is_async: false,
            type_params: Vec::new(),
            behavior_attrs: BTreeMap::new(),
            params: vec![RuntimeParamPlan {
                binding_id: 0,
                mode: Some("edit".to_string()),
                name: "value".to_string(),
                ty: parse_routine_type_text("Int").expect("type"),
            }],
            return_type: test_return_type("fn bump(edit value: Int) -> Int:"),
            intrinsic_impl: None,
            native_impl: None,
            impl_target_type: None,
            impl_trait_path: None,
            availability: Vec::new(),
            cleanup_footers: Vec::new(),
            statements: vec![
                ParsedStmt::Assign {
                    target: ParsedAssignTarget::Name("value".to_string()),
                    op: ParsedAssignOp::AddAssign,
                    value: ParsedExpr::Int(2),
                },
                ParsedStmt::ReturnValue {
                    value: ParsedExpr::Path(vec!["value".to_string()]),
                },
            ],
        }],
    };
    let mut host = BufferedHost::default();
    let result = execute_exported_abi_routine(
        &plan,
        "tool#fn-0",
        vec![super::RuntimeAbiValue::Int(5)],
        &mut host,
    )
    .expect("native abi invoke should succeed");
    assert_eq!(result.result, super::RuntimeAbiValue::Int(7));
    assert_eq!(
        result.write_backs,
        vec![super::RuntimeAbiWriteBack {
            index: 0,
            name: "value".to_string(),
            value: super::RuntimeAbiValue::Int(7),
        }]
    );
}

#[test]
fn execute_main_rejects_missing_runtime_requirement() {
    let plan = plan_from_artifact(&sample_print_artifact()).expect("runtime plan should build");
    let mut host = BufferedHost {
        supported_runtime_requirements: Some(
            ["arcana_process.args".to_string()].into_iter().collect(),
        ),
        ..BufferedHost::default()
    };
    let err = execute_main(&plan, &mut host).expect_err("runtime should reject missing io");
    assert!(
        err.contains("arcana_process.io"),
        "expected arcana_process.io capability error, got {err}"
    );
}

#[test]
fn execute_entrypoint_routine_runs_named_main_by_routine_key() {
    let plan = plan_from_artifact(&sample_return_artifact()).expect("runtime plan should build");
    let mut host = BufferedHost::default();
    let code = execute_entrypoint_routine(&plan, "hello#sym-0", &mut host)
        .expect("named entrypoint routine should execute");
    assert_eq!(code, 7);
}

#[test]
fn execute_routine_rejects_missing_runtime_requirement() {
    let plan = plan_from_artifact(&sample_return_artifact()).expect("runtime plan should build");
    let mut host = BufferedHost {
        supported_runtime_requirements: Some(BTreeSet::new()),
        ..BufferedHost::default()
    };

    let err = execute_routine(&plan, 0, Vec::new(), &mut host)
        .expect_err("runtime should reject missing io");
    assert!(
        err.contains("arcana_process.io"),
        "expected arcana_process.io capability error, got {err}"
    );
}

#[test]
fn plan_from_artifact_accepts_stmt_forewords_and_cleanup_footers() {
    let plan =
        plan_from_artifact(&sample_stmt_metadata_artifact()).expect("runtime plan should build");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");
    assert_eq!(code, 0, "stdout={:?}", host.stdout);
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
fn runtime_plan_exposes_runtime_retained_foreword_index() {
    let mut artifact = sample_return_artifact();
    artifact.foreword_index = vec![
        IrForewordMetadata {
            entry_kind: arcana_ir::IrForewordEntryKind::Attached,
            qualified_name: "tool.meta.cache".to_string(),
            package_id: test_package_id_for_module("hello"),
            module_id: "hello".to_string(),
            target_kind: "fn".to_string(),
            target_path: "hello.main".to_string(),
            retention: IrForewordRetention::Runtime,
            args: vec![IrForewordArg {
                name: Some("slot".to_string()),
                value: "\"menu\"".to_string(),
                typed_value: arcana_ir::IrForewordArgValue::Str("menu".to_string()),
            }],
            public: true,
            generated_by: None,
        },
        IrForewordMetadata {
            entry_kind: arcana_ir::IrForewordEntryKind::Attached,
            qualified_name: "tool.meta.secret".to_string(),
            package_id: test_package_id_for_module("hello"),
            module_id: "hello".to_string(),
            target_kind: "fn".to_string(),
            target_path: "hello.secret".to_string(),
            retention: IrForewordRetention::Runtime,
            args: Vec::new(),
            public: false,
            generated_by: None,
        },
    ];

    let plan = plan_from_artifact(&artifact).expect("runtime plan should build");
    assert_eq!(plan.forewords().len(), 2);
    assert_eq!(plan.runtime_retained_forewords().len(), 2);
    assert_eq!(plan.public_runtime_retained_forewords().len(), 1);
    assert_eq!(
        plan.runtime_retained_forewords_for_target("fn", "hello.main")
            .len(),
        1
    );
    assert_eq!(
        plan.public_runtime_retained_forewords_for_target("fn", "hello.main")
            .len(),
        1
    );
    assert_eq!(
        plan.public_runtime_retained_forewords_for_target("fn", "hello.secret")
            .len(),
        0
    );
    assert_eq!(
        plan.runtime_retained_forewords_for_target("fn", "hello.main")[0].args[0]
            .name
            .as_deref(),
        Some("slot")
    );
}

#[test]
fn runtime_plan_preserves_generated_foreword_provenance() {
    let mut artifact = sample_return_artifact();
    artifact.foreword_index = vec![IrForewordMetadata {
        entry_kind: arcana_ir::IrForewordEntryKind::Generated,
        qualified_name: "tool.exec.rewrite".to_string(),
        package_id: test_package_id_for_module("hello"),
        module_id: "hello".to_string(),
        target_kind: "fn".to_string(),
        target_path: "hello.helper".to_string(),
        retention: IrForewordRetention::Tooling,
        args: Vec::new(),
        public: true,
        generated_by: Some(arcana_ir::IrForewordGeneratedBy {
            applied_name: "tool.exec.rewrite".to_string(),
            resolved_name: "tool.exec.rewrite".to_string(),
            provider_package_id: "tool.pkg".to_string(),
            owner_kind: "fn".to_string(),
            owner_path: "hello.main".to_string(),
            retention: IrForewordRetention::Tooling,
            args: Vec::new(),
        }),
    }];

    let plan = plan_from_artifact(&artifact).expect("runtime plan should build");
    let entry = plan
        .forewords()
        .first()
        .expect("generated provenance entry should exist");
    assert_eq!(entry.entry_kind, arcana_ir::IrForewordEntryKind::Generated);
    let generated_by = entry
        .generated_by
        .as_ref()
        .expect("generated provenance should survive");
    assert_eq!(generated_by.owner_path, "hello.main");
    assert_eq!(generated_by.provider_package_id, "tool.pkg");
}

#[test]
fn runtime_plan_exposes_foreword_registrations() {
    let mut artifact = sample_return_artifact();
    artifact.foreword_registrations = vec![
        arcana_ir::IrForewordRegistrationRow {
            namespace: "tool.exec.registry".to_string(),
            key: "helper".to_string(),
            value: "runtime".to_string(),
            target_kind: "fn".to_string(),
            target_path: "hello.helper".to_string(),
            public: true,
            generated_by: arcana_ir::IrForewordGeneratedBy {
                applied_name: "tool.exec.rewrite".to_string(),
                resolved_name: "tool.exec.rewrite".to_string(),
                provider_package_id: "tool.pkg".to_string(),
                owner_kind: "fn".to_string(),
                owner_path: "hello.main".to_string(),
                retention: IrForewordRetention::Tooling,
                args: Vec::new(),
            },
        },
        arcana_ir::IrForewordRegistrationRow {
            namespace: "tool.exec.registry".to_string(),
            key: "private".to_string(),
            value: "hidden".to_string(),
            target_kind: "fn".to_string(),
            target_path: "hello.secret".to_string(),
            public: false,
            generated_by: arcana_ir::IrForewordGeneratedBy {
                applied_name: "tool.exec.rewrite".to_string(),
                resolved_name: "tool.exec.rewrite".to_string(),
                provider_package_id: "tool.pkg".to_string(),
                owner_kind: "fn".to_string(),
                owner_path: "hello.main".to_string(),
                retention: IrForewordRetention::Tooling,
                args: Vec::new(),
            },
        },
    ];

    let plan = plan_from_artifact(&artifact).expect("runtime plan should build");
    assert_eq!(plan.foreword_registrations().len(), 2);
    assert_eq!(plan.public_foreword_registrations().len(), 1);
    assert_eq!(
        plan.foreword_registrations_for_target("fn", "hello.helper")
            .len(),
        1
    );
    assert_eq!(
        plan.public_foreword_registrations_for_target("fn", "hello.helper")
            .len(),
        1
    );
    assert_eq!(
        plan.public_foreword_registrations_for_target("fn", "hello.secret")
            .len(),
        0
    );
    let row = plan
        .public_foreword_registrations()
        .into_iter()
        .next()
        .expect("public registration row should exist");
    assert_eq!(row.namespace, "tool.exec.registry");
    assert_eq!(row.key, "helper");
    assert_eq!(row.target_path, "hello.helper");
    let generated_by = &row.generated_by;
    assert_eq!(generated_by.owner_path, "hello.main");
    assert_eq!(generated_by.provider_package_id, "tool.pkg");
}

#[test]
fn runtime_plan_supports_package_id_stable_foreword_lookup() {
    let mut artifact = sample_return_artifact();
    artifact
        .package_display_names
        .insert("dep.pkg".to_string(), "dep".to_string());
    artifact.modules.push(AotPackageModuleArtifact {
        package_id: "dep.pkg".to_string(),
        module_id: "dep.util".to_string(),
        symbol_count: 1,
        item_count: 1,
        line_count: 1,
        non_empty_line_count: 1,
        directive_rows: Vec::new(),
        lang_item_rows: Vec::new(),
        exported_surface_rows: Vec::new(),
    });
    artifact.module_count = artifact.modules.len();
    artifact.foreword_index = vec![
        IrForewordMetadata {
            entry_kind: arcana_ir::IrForewordEntryKind::Attached,
            qualified_name: "tool.meta.cache".to_string(),
            package_id: test_package_id_for_module("hello"),
            module_id: "hello".to_string(),
            target_kind: "fn".to_string(),
            target_path: "hello.main".to_string(),
            retention: IrForewordRetention::Runtime,
            args: Vec::new(),
            public: true,
            generated_by: None,
        },
        IrForewordMetadata {
            entry_kind: arcana_ir::IrForewordEntryKind::Attached,
            qualified_name: "tool.meta.cache".to_string(),
            package_id: "dep.pkg".to_string(),
            module_id: "dep.util".to_string(),
            target_kind: "fn".to_string(),
            target_path: "dep.util.helper".to_string(),
            retention: IrForewordRetention::Runtime,
            args: Vec::new(),
            public: false,
            generated_by: None,
        },
    ];

    let plan = plan_from_artifact(&artifact).expect("runtime plan should build");
    assert_eq!(
        plan.runtime_retained_forewords_for_package("hello").len(),
        1
    );
    assert_eq!(
        plan.public_runtime_retained_forewords_for_package("hello")
            .len(),
        1
    );
    assert_eq!(
        plan.runtime_retained_forewords_for_package("dep.pkg").len(),
        1
    );
    assert_eq!(
        plan.public_runtime_retained_forewords_for_package("dep.pkg")
            .len(),
        0
    );
}

#[test]
fn execute_main_runs_cleanup_footers_on_loop_exit_and_try_propagation() {
    let dir = temp_workspace_dir("cleanup_footers");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_cleanup_footers\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import arcana_process.io\n",
            "import std.result\n",
            "use std.result.Result\n",
            "record Scratch:\n",
            "    value: Int\n",
            "impl std.cleanup.Cleanup[Scratch] for Scratch:\n",
            "    fn cleanup(take self: Scratch) -> Result[Unit, Str]:\n",
            "        return Result.Ok[Unit, Str] :: :: call\n",
            "fn cleanup(take value: Scratch) -> Result[Unit, Str]:\n",
            "    arcana_process.io.print[Int] :: value.value :: call\n",
            "    return Result.Ok[Unit, Str] :: :: call\n",
            "fn maybe(flag: Bool) -> Result[Int, Str]:\n",
            "    if flag:\n",
            "        return Result.Err[Int, Str] :: \"bad\" :: call\n",
            "    return Result.Ok[Int, Str] :: 9 :: call\n",
            "fn run(seed: Int, flag: Bool) -> Result[Int, Str]:\n",
            "    let mut local = seed\n",
            "    while local > 0:\n",
            "        let scratch = Scratch :: value = local :: call\n",
            "        local -= 1\n",
            "    -cleanup[target = scratch, handler = cleanup]\n",
            "    let value = (maybe :: flag :: call) :: :: ?\n",
            "    return Result.Ok[Int, Str] :: value :: call\n",
            "fn main() -> Int:\n",
            "    let result = run :: 2, true :: call\n",
            "    arcana_process.io.print[Bool] :: (result :: :: is_err) :: call\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_cleanup_footers")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");
    assert_eq!(code, 0);
    assert_eq!(
        host.stdout,
        vec!["2".to_string(), "1".to_string(), "true".to_string(),]
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn build_workspace_rejects_ambiguous_cleanup_footer_target_under_shadowing() {
    let dir = temp_workspace_dir("cleanup_footer_shadowing");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_cleanup_footer_shadowing\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import arcana_process.io\n",
            "import std.result\n",
            "use std.result.Result\n",
            "record Box:\n",
            "    value: Int\n",
            "impl std.cleanup.Cleanup[Box] for Box:\n",
            "    fn cleanup(take self: Box) -> Result[Unit, Str]:\n",
            "        return Result.Ok[Unit, Str] :: :: call\n",
            "fn cleanup(take value: Box) -> Result[Unit, Str]:\n",
            "    arcana_process.io.print[Int] :: value.value :: call\n",
            "    return Result.Ok[Unit, Str] :: :: call\n",
            "fn main() -> Int:\n",
            "    let x = Box :: value = 1 :: call\n",
            "    if true:\n",
            "        let x = Box :: value = 2 :: call\n",
            "    return 0\n",
            "-cleanup[target = x, handler = cleanup]\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let err = match load_workspace_graph(&dir).and_then(|graph| check_workspace_graph(&graph)) {
        Ok(_) => panic!("shadowed cleanup footer target should be ambiguous"),
        Err(err) => err,
    };
    assert!(
        err.contains("cleanup footer target `x` is ambiguous in the owning header scope"),
        "unexpected error: {err}"
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_cleanup_footers_refresh_subject_value_after_mutation() {
    let dir = temp_workspace_dir("cleanup_footer_mutation_refresh");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_cleanup_footer_mutation_refresh\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import arcana_process.io\n",
            "import std.result\n",
            "use std.result.Result\n",
            "use types.Counter\n",
            "impl std.cleanup.Cleanup[Counter] for Counter:\n",
            "    fn cleanup(take self: Counter) -> Result[Unit, Str]:\n",
            "        return Result.Ok[Unit, Str] :: :: call\n",
            "fn cleanup(take counter: Counter) -> Result[Unit, Str]:\n",
            "    arcana_process.io.print[Int] :: counter.value :: call\n",
            "    return Result.Ok[Unit, Str] :: :: call\n",
            "fn main() -> Int:\n",
            "    let mut counter = Counter :: value = 1 :: call\n",
            "    counter.value = 2\n",
            "    return 0\n",
            "-cleanup[target = counter, handler = cleanup]\n",
        ),
    );
    write_file(
        &dir.join("src").join("types.arc"),
        "export record Counter:\n    value: Int\n",
    );

    let plan = build_workspace_plan_for_member(&dir, "runtime_cleanup_footer_mutation_refresh");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(host.stdout, vec!["2".to_string()]);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_bare_cleanup_footer_covers_whole_routine_scope() {
    let dir = temp_workspace_dir("bare_cleanup_footer_scope");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_bare_cleanup_footer_scope\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import arcana_process.io\n",
            "import std.result\n",
            "use std.result.Result\n",
            "record Counter:\n",
            "    value: Int\n",
            "impl std.cleanup.Cleanup[Counter] for Counter:\n",
            "    fn cleanup(take self: Counter) -> Result[Unit, Str]:\n",
            "        arcana_process.io.print[Int] :: self.value :: call\n",
            "        return Result.Ok[Unit, Str] :: :: call\n",
            "fn main() -> Int:\n",
            "    let first = Counter :: value = 1 :: call\n",
            "    let second = Counter :: value = 2 :: call\n",
            "    return 0\n",
            "-cleanup\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let plan = build_workspace_plan_for_member(&dir, "runtime_bare_cleanup_footer_scope");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(host.stdout, vec!["2".to_string(), "1".to_string()]);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_bare_cleanup_footer_covers_nested_scope_bindings() {
    let dir = temp_workspace_dir("bare_cleanup_footer_nested_scope");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_bare_cleanup_footer_nested_scope\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import arcana_process.io\n",
            "import std.result\n",
            "use std.result.Result\n",
            "record Counter:\n",
            "    value: Int\n",
            "impl std.cleanup.Cleanup[Counter] for Counter:\n",
            "    fn cleanup(take self: Counter) -> Result[Unit, Str]:\n",
            "        arcana_process.io.print[Int] :: self.value :: call\n",
            "        return Result.Ok[Unit, Str] :: :: call\n",
            "fn main() -> Int:\n",
            "    let outer = Counter :: value = 1 :: call\n",
            "    if true:\n",
            "        let inner = Counter :: value = 2 :: call\n",
            "    return 0\n",
            "-cleanup\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let plan = build_workspace_plan_for_member(&dir, "runtime_bare_cleanup_footer_nested_scope");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(host.stdout, vec!["2".to_string(), "1".to_string()]);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_cleanup_footer_targets_nested_scope_binding() {
    let dir = temp_workspace_dir("cleanup_footer_nested_target");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_cleanup_footer_nested_target\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import arcana_process.io\n",
            "import std.result\n",
            "use std.result.Result\n",
            "record Box:\n",
            "    value: Int\n",
            "impl std.cleanup.Cleanup[Box] for Box:\n",
            "    fn cleanup(take self: Box) -> Result[Unit, Str]:\n",
            "        return Result.Ok[Unit, Str] :: :: call\n",
            "fn cleanup(take value: Box) -> Result[Unit, Str]:\n",
            "    arcana_process.io.print[Int] :: value.value :: call\n",
            "    return Result.Ok[Unit, Str] :: :: call\n",
            "fn main() -> Int:\n",
            "    if true:\n",
            "        let inner = Box :: value = 2 :: call\n",
            "    return 0\n",
            "-cleanup[target = inner, handler = cleanup]\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let plan = build_workspace_plan_for_member(&dir, "runtime_cleanup_footer_nested_target");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(host.stdout, vec!["2".to_string()]);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_manual_routine_cleanup_footers_run_after_defers() {
    let plan = RuntimePackagePlan {
        package_id: "manual_routine_cleanup_footers".to_string(),
        package_name: "manual_routine_cleanup_footers".to_string(),
        root_module_id: "manual_routine_cleanup_footers".to_string(),
        direct_deps: Vec::new(),
        direct_dep_ids: Vec::new(),
        package_display_names: test_package_display_names_with_deps(
            "manual_routine_cleanup_footers".to_string(),
            "manual_routine_cleanup_footers".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        package_direct_dep_ids: test_package_direct_dep_ids(
            "manual_routine_cleanup_footers".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        runtime_requirements: Vec::new(),
        foreword_index: Vec::new(),
        foreword_registrations: Vec::new(),
        module_aliases: BTreeMap::new(),
        opaque_family_types: BTreeMap::new(),
        entrypoints: vec![RuntimeEntrypointPlan {
            package_id: test_package_id_for_module("manual_routine_cleanup_footers"),
            module_id: "manual_routine_cleanup_footers".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            is_async: false,
            exported: true,
            routine_index: 1,
        }],
        native_callbacks: Vec::new(),
        shackle_decls: Vec::new(),
        binding_layouts: Vec::new(),
        owners: Vec::new(),
        routines: vec![
            RuntimeRoutinePlan {
                package_id: test_package_id_for_module("manual_routine_cleanup_footers"),
                module_id: "manual_routine_cleanup_footers".to_string(),
                routine_key: "manual_routine_cleanup_footers#sym-0".to_string(),
                symbol_name: "run".to_string(),
                symbol_kind: "fn".to_string(),
                exported: false,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: vec![RuntimeParamPlan {
                    binding_id: 0,
                    mode: Some("read".to_string()),
                    name: "seed".to_string(),
                    ty: parse_routine_type_text("Int").expect("type"),
                }],
                return_type: test_return_type("fn run(read seed: Int) -> Result[Int, Str]:"),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                cleanup_footers: vec![ParsedCleanupFooter {
                    binding_id: 0,
                    kind: "cleanup".to_string(),
                    subject: "seed".to_string(),
                    handler_path: vec!["std".to_string(), "io".to_string(), "print".to_string()],
                    resolved_routine: None,
                }],
                statements: vec![
                    ParsedStmt::Defer(arcana_ir::ExecDeferAction::Expr(ParsedExpr::Phrase {
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
                        qualifier_type_args: Vec::new(),
                        resolved_callable: None,
                        resolved_routine: None,
                        dynamic_dispatch: None,
                        attached: Vec::new(),
                    })),
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
                                qualifier_type_args: Vec::new(),
                                resolved_callable: None,
                                resolved_routine: None,
                                dynamic_dispatch: None,
                                attached: Vec::new(),
                            }),
                            args: Vec::new(),
                            qualifier_kind: ParsedPhraseQualifierKind::Try,
                            qualifier: "?".to_string(),
                            qualifier_type_args: Vec::new(),
                            resolved_callable: None,
                            resolved_routine: None,
                            dynamic_dispatch: None,
                            attached: Vec::new(),
                        },
                        cleanup_footers: Vec::new(),
                    },
                ],
            },
            RuntimeRoutinePlan {
                package_id: test_package_id_for_module("manual_routine_cleanup_footers"),
                module_id: "manual_routine_cleanup_footers".to_string(),
                routine_key: "manual_routine_cleanup_footers#sym-1".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: Vec::new(),
                return_type: test_return_type("fn main() -> Int:"),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                cleanup_footers: Vec::new(),
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
                            qualifier_type_args: Vec::new(),
                            resolved_callable: None,
                            resolved_routine: None,
                            dynamic_dispatch: None,
                            attached: Vec::new(),
                        },
                        cleanup_footers: Vec::new(),
                    },
                    ParsedStmt::ReturnValue {
                        value: ParsedExpr::Int(0),
                    },
                ],
            },
            RuntimeRoutinePlan {
                package_id: test_package_id_for_module("arcana_process.io"),
                module_id: "arcana_process.io".to_string(),
                routine_key: "arcana_process.io#sym-0".to_string(),
                symbol_name: "print".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_params: vec!["T".to_string()],
                behavior_attrs: BTreeMap::new(),
                params: vec![RuntimeParamPlan {
                    binding_id: 0,
                    mode: Some("read".to_string()),
                    name: "value".to_string(),
                    ty: parse_routine_type_text("T").expect("type"),
                }],
                return_type: test_return_type("fn print[T](read value: T):"),
                intrinsic_impl: Some("IoPrint".to_string()),
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                cleanup_footers: Vec::new(),
                statements: Vec::new(),
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
            "import arcana_process.io\n",
            "use arcana_process.io as io\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_counter")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
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
            "import arcana_process.args\n",
            "import arcana_process.io\n",
            "fn add_one(value: Int) -> Int:\n",
            "    return value + 1\n",
            "fn main() -> Int:\n",
            "    let argc = arcana_process.args.count :: :: call\n",
            "    let total = add_one :: argc :: call\n",
            "    arcana_process.io.print[Int] :: total :: call\n",
            "    if argc > 0:\n",
            "        let first = arcana_process.args.get :: 0 :: call\n",
            "        arcana_process.io.print[Str] :: first :: call\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_args")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
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
            "import arcana_process.io\n",
            "import std.text\n",
            "fn main() -> Int:\n",
            "    arcana_process.io.print[Int] :: (std.text.find :: \"abc\", 0, \"b\" :: call) :: call\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_std_text")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
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
            "import arcana_process.io\n",
            "fn main() -> Int:\n",
            "    let mut values = std.collections.list.new[Int] :: :: call\n",
            "    values :: 4 :: push\n",
            "    values :: 9 :: push\n",
            "    let arr = std.collections.array.from_list[Int] :: values :: call\n",
            "    let mut sum = 0\n",
            "    for value in arr:\n",
            "        sum += value\n",
            "    arcana_process.io.print[Int] :: (arr :: :: len) :: call\n",
            "    arcana_process.io.print[Int] :: sum :: call\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_std_array")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
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
            "import arcana_process.io\n",
            "import std.iter\n",
            "fn main() -> Int:\n",
            "    let mut it = std.iter.range :: 2, 5 :: call\n",
            "    arcana_process.io.print[Int] :: (std.iter.count[std.iter.RangeIter] :: it :: call) :: call\n",
            "    let mut xs = std.collections.set.new[Int] :: :: call\n",
            "    arcana_process.io.print[Bool] :: (xs :: 7 :: insert) :: call\n",
            "    arcana_process.io.print[Bool] :: (xs :: 7 :: insert) :: call\n",
            "    arcana_process.io.print[Bool] :: (xs :: 7 :: has) :: call\n",
            "    arcana_process.io.print[Int] :: (xs :: :: len) :: call\n",
            "    arcana_process.io.print[Bool] :: (xs :: 7 :: remove) :: call\n",
            "    arcana_process.io.print[Int] :: (xs :: :: len) :: call\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_std_iter_set")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0, "stdout={:?}", host.stdout);
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
            "import arcana_process.io\n",
            "fn main() -> Int:\n",
            "    let text = \"name = \\\"Arcana\\\"\\n[deps]\\nfoo = { path = \\\"../foo\\\" }\\n[settings]\\nmode = \\\"dev\\\"\\n\"\n",
            "    let parsed = std.config.parse_document :: text :: call\n",
            "    if parsed :: :: is_err:\n",
            "        arcana_process.io.print[Str] :: (parsed :: \"parse error\" :: unwrap_or) :: call\n",
            "        return 1\n",
            "    let doc = parsed :: (std.config.empty_document :: :: call) :: unwrap_or\n",
            "    arcana_process.io.print[Bool] :: (doc :: \"name\" :: root_has_key) :: call\n",
            "    arcana_process.io.print[Bool] :: (doc :: \"settings\" :: has_section) :: call\n",
            "    arcana_process.io.print[Str] :: ((doc :: \"name\", \"config field\" :: root_required_string) :: \"missing\" :: unwrap_or) :: call\n",
            "    arcana_process.io.print[Str] :: ((doc :: \"settings\", \"mode\", \"settings field\" :: section_required) :: \"missing\" :: unwrap_or) :: call\n",
            "    arcana_process.io.print[Str] :: ((doc :: \"deps\", (\"foo\", \"path\"), \"dependency entry\" :: section_inline_table_string_field) :: \"missing\" :: unwrap_or) :: call\n",
            "    arcana_process.io.print[Int] :: ((doc :: \"settings\" :: entries_in_section) :: :: len) :: call\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_std_config")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
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
            "import arcana_process.io\n",
            "import std.manifest\n",
            "fn main() -> Int:\n",
            "    let book = \"name = \\\"demo\\\"\\nkind = \\\"app\\\"\\nversion = \\\"0.1.0\\\"\\n[workspace]\\nmembers = [\\\"game\\\", \\\"tools\\\"]\\n[deps]\\nfoo = { version = \\\"^1.2.3\\\", registry = \\\"local\\\" }\\nbar = { path = \\\"../bar\\\" }\\n\"\n",
            "    let parsed_book = std.manifest.parse_book :: book :: call\n",
            "    if parsed_book :: :: is_err:\n",
            "        let err = match parsed_book:\n",
            "            Result.Ok(_) => \"book parse error\"\n",
            "            Result.Err(message) => message\n",
            "        arcana_process.io.print[Str] :: err :: call\n",
            "        return 1\n",
            "    let book_manifest = parsed_book :: (std.manifest.empty_book_manifest :: :: call) :: unwrap_or\n",
            "    let members = book_manifest :: :: workspace_members\n",
            "    arcana_process.io.print[Int] :: ((members :: (std.collections.list.new[Str] :: :: call) :: unwrap_or) :: :: len) :: call\n",
            "    arcana_process.io.print[Str] :: book_manifest.package_version :: call\n",
            "    arcana_process.io.print[Str] :: ((book_manifest :: \"foo\" :: dep_source_kind) :: \"missing\" :: unwrap_or) :: call\n",
            "    arcana_process.io.print[Str] :: ((book_manifest :: \"foo\" :: dep_version) :: \"missing\" :: unwrap_or) :: call\n",
            "    arcana_process.io.print[Str] :: ((book_manifest :: \"foo\" :: dep_registry) :: \"missing\" :: unwrap_or) :: call\n",
            "    arcana_process.io.print[Str] :: ((book_manifest :: \"bar\" :: dep_path) :: \"missing\" :: unwrap_or) :: call\n",
            "    let lock_v1 = \"version = 1\\nworkspace = \\\"demo\\\"\\norder = [\\\"game\\\", \\\"tools\\\"]\\n[deps]\\ngame = [\\\"foo\\\", \\\"bar\\\"]\\n[paths]\\ngame = \\\"grimoires/owned/app/game\\\"\\n[fingerprints]\\ngame = \\\"fp1\\\"\\n[api_fingerprints]\\ngame = \\\"api1\\\"\\n[artifacts]\\ngame = \\\"build/app.artifact.toml\\\"\\n[kinds]\\ngame = \\\"app\\\"\\n[formats]\\ngame = \\\"arcana-aot-v2\\\"\\n\"\n",
            "    let parsed_lock_v1 = std.manifest.parse_lock_v1 :: lock_v1 :: call\n",
            "    if parsed_lock_v1 :: :: is_err:\n",
            "        arcana_process.io.print[Str] :: \"lock v1 parse error\" :: call\n",
            "        return 1\n",
            "    let empty_metadata = std.manifest.empty_lock_metadata :: :: call\n",
            "    let empty_deps = std.manifest.LockDependencyTables :: dependency_lists = (std.collections.list.new[std.manifest.NameList] :: :: call), path_entries = (std.collections.list.new[std.manifest.NameValue] :: :: call), fingerprint_entries = (std.collections.list.new[std.manifest.NameValue] :: :: call) :: call\n",
            "    let empty_lookup = std.manifest.LockLookupTables :: dependencies = empty_deps, api_fingerprint_entries = (std.collections.list.new[std.manifest.NameValue] :: :: call) :: call\n",
            "    let empty_output = std.manifest.LockOutputTables :: artifact_entries = (std.collections.list.new[std.manifest.NameValue] :: :: call), kind_entries = (std.collections.list.new[std.manifest.NameValue] :: :: call), format_entries = (std.collections.list.new[std.manifest.NameValue] :: :: call) :: call\n",
            "    let empty_members = std.manifest.empty_lock_member_tables :: :: call\n",
            "    let empty_builds = std.manifest.empty_lock_build_tables :: :: call\n",
            "    let lock_manifest_v1 = parsed_lock_v1 :: (std.manifest.LockManifestV1 :: metadata = empty_metadata, lookup_tables = empty_lookup, output_tables = empty_output :: call) :: unwrap_or\n",
            "    let deps = lock_manifest_v1 :: \"game\" :: deps_for\n",
            "    arcana_process.io.print[Int] :: ((deps :: (std.collections.list.new[Str] :: :: call) :: unwrap_or) :: :: len) :: call\n",
            "    arcana_process.io.print[Str] :: ((lock_manifest_v1 :: \"game\" :: path_for) :: \"missing\" :: unwrap_or) :: call\n",
            "    arcana_process.io.print[Str] :: ((lock_manifest_v1 :: \"game\" :: kind_for) :: \"missing\" :: unwrap_or) :: call\n",
            "    arcana_process.io.print[Str] :: ((lock_manifest_v1 :: \"game\" :: format_for) :: \"missing\" :: unwrap_or) :: call\n",
            "    let parsed_lock_generic_v1 = std.manifest.parse_lock :: lock_v1 :: call\n",
            "    if parsed_lock_generic_v1 :: :: is_err:\n",
            "        arcana_process.io.print[Str] :: \"lock generic v1 parse error\" :: call\n",
            "        return 1\n",
            "    let lock_manifest_generic_v1 = parsed_lock_generic_v1 :: (std.manifest.LockManifestV2 :: metadata = empty_metadata, member_tables = empty_members, build_tables = empty_builds :: call) :: unwrap_or\n",
            "    arcana_process.io.print[Str] :: ((lock_manifest_generic_v1 :: \"game\" :: source_kind_for) :: \"missing\" :: unwrap_or) :: call\n",
            "    arcana_process.io.print[Str] :: ((lock_manifest_generic_v1 :: \"game\", \"internal-aot\" :: format_for) :: \"missing\" :: unwrap_or) :: call\n",
            "    let lock_v3 = \"version = 3\\nworkspace = \\\"demo\\\"\\norder = [\\\"game\\\"]\\n[paths]\\ngame = \\\"grimoires/owned/app/game\\\"\\n[deps]\\ngame = [\\\"foo\\\"]\\n[kinds]\\ngame = \\\"app\\\"\\n[native_products]\\n\\n[native_products.\\\"game\\\".\\\"default\\\"]\\nkind = \\\"cdylib\\\"\\nrole = \\\"export\\\"\\nproducer = \\\"rust\\\"\\nfile = \\\"game.dll\\\"\\ncontract = \\\"arcana-desktop-v1\\\"\\nsidecars = [\\\"game.pdb\\\"]\\n\\n[builds]\\n\\n[builds.\\\"game\\\".\\\"internal-aot\\\"]\\nfingerprint = \\\"fp3\\\"\\napi_fingerprint = \\\"api3\\\"\\nartifact = \\\".arcana/artifacts/game/internal-aot/app.artifact.toml\\\"\\nartifact_hash = \\\"hash3\\\"\\nformat = \\\"arcana-aot-v7\\\"\\ntoolchain = \\\"toolchain-1\\\"\\n\"\n",
            "    let parsed_lock_v3 = std.manifest.parse_lock :: lock_v3 :: call\n",
            "    if parsed_lock_v3 :: :: is_err:\n",
            "        arcana_process.io.print[Str] :: \"lock v3 parse error\" :: call\n",
            "        return 1\n",
            "    let lock_manifest_v3 = parsed_lock_v3 :: (std.manifest.LockManifestV2 :: metadata = empty_metadata, member_tables = empty_members, build_tables = empty_builds :: call) :: unwrap_or\n",
            "    arcana_process.io.print[Str] :: ((lock_manifest_v3 :: \"game\" :: source_kind_for) :: \"missing\" :: unwrap_or) :: call\n",
            "    arcana_process.io.print[Str] :: ((lock_manifest_v3 :: \"game\", \"default\" :: native_product_kind_for) :: \"missing\" :: unwrap_or) :: call\n",
            "    let lock_v2 = \"version = 4\\nworkspace = \\\"demo\\\"\\nworkspace_root = \\\"path:game\\\"\\norder = [\\\"path:game\\\", \\\"path:tools\\\", \\\"registry:local:foo@1.2.3\\\", \\\"git:https://example.com/arcana/tooling.git#tag:v1.2.3:tooling\\\"]\\nworkspace_members = [\\\"path:game\\\", \\\"path:tools\\\"]\\n[packages]\\n\\n[packages.\\\"path:game\\\"]\\nname = \\\"game\\\"\\nkind = \\\"app\\\"\\nsource_kind = \\\"path\\\"\\npath = \\\"grimoires/owned/app/game\\\"\\n\\n[packages.\\\"path:tools\\\"]\\nname = \\\"tools\\\"\\nkind = \\\"lib\\\"\\nsource_kind = \\\"path\\\"\\npath = \\\"grimoires/owned/app/tools\\\"\\n\\n[packages.\\\"registry:local:foo@1.2.3\\\"]\\nname = \\\"foo\\\"\\nkind = \\\"lib\\\"\\nsource_kind = \\\"registry\\\"\\nversion = \\\"1.2.3\\\"\\nregistry = \\\"local\\\"\\nchecksum = \\\"sha256:abc123\\\"\\n\\n[packages.\\\"git:https://example.com/arcana/tooling.git#tag:v1.2.3:tooling\\\"]\\nname = \\\"tooling\\\"\\nkind = \\\"lib\\\"\\nsource_kind = \\\"git\\\"\\ngit = \\\"https://example.com/arcana/tooling.git\\\"\\ngit_selector = \\\"tag:v1.2.3\\\"\\n\\n[dependencies]\\n\\n[dependencies.\\\"path:game\\\"]\\nfoo = \\\"registry:local:foo@1.2.3\\\"\\nbar = \\\"path:tools\\\"\\n\\n[dependencies.\\\"path:tools\\\"]\\n\\n[dependencies.\\\"registry:local:foo@1.2.3\\\"]\\n\\n[dependencies.\\\"git:https://example.com/arcana/tooling.git#tag:v1.2.3:tooling\\\"]\\n\\n[native_products]\\n\\n[native_products.\\\"path:game\\\".\\\"default\\\"]\\nkind = \\\"cdylib\\\"\\nrole = \\\"export\\\"\\nproducer = \\\"rust\\\"\\nfile = \\\"game.dll\\\"\\ncontract = \\\"arcana-desktop-v1\\\"\\nrust_cdylib_crate = \\\"arcana_game\\\"\\nsidecars = [\\\"game.pdb\\\", \\\"game.json\\\"]\\n\\n[builds]\\n\\n[builds.\\\"path:game\\\".\\\"internal-aot\\\"]\\nfingerprint = \\\"fp2\\\"\\napi_fingerprint = \\\"api2\\\"\\nartifact = \\\".arcana/artifacts/game/internal-aot/app.artifact.toml\\\"\\nartifact_hash = \\\"hash2\\\"\\nformat = \\\"arcana-aot-v8\\\"\\ntoolchain = \\\"toolchain-1\\\"\\n\\n[builds.\\\"path:tools\\\".\\\"internal-aot\\\"]\\nfingerprint = \\\"fp3\\\"\\napi_fingerprint = \\\"api3\\\"\\nartifact = \\\".arcana/artifacts/tools/internal-aot/lib.artifact.toml\\\"\\nartifact_hash = \\\"hash3\\\"\\nformat = \\\"arcana-aot-v8\\\"\\ntoolchain = \\\"toolchain-1\\\"\\n\\n[builds.\\\"registry:local:foo@1.2.3\\\".\\\"internal-aot\\\"]\\nfingerprint = \\\"fp4\\\"\\napi_fingerprint = \\\"api4\\\"\\nartifact = \\\".arcana/artifacts/foo/internal-aot/lib.artifact.toml\\\"\\nartifact_hash = \\\"hash4\\\"\\nformat = \\\"arcana-aot-v8\\\"\\ntoolchain = \\\"toolchain-1\\\"\\n\\n[builds.\\\"git:https://example.com/arcana/tooling.git#tag:v1.2.3:tooling\\\".\\\"internal-aot\\\"]\\nfingerprint = \\\"fp5\\\"\\napi_fingerprint = \\\"api5\\\"\\nartifact = \\\".arcana/artifacts/tooling/internal-aot/lib.artifact.toml\\\"\\nartifact_hash = \\\"hash5\\\"\\nformat = \\\"arcana-aot-v8\\\"\\ntoolchain = \\\"toolchain-1\\\"\\n\"\n",
            "    let parsed_lock_v2 = std.manifest.parse_lock :: lock_v2 :: call\n",
            "    if parsed_lock_v2 :: :: is_err:\n",
            "        arcana_process.io.print[Str] :: \"lock v2 parse error\" :: call\n",
            "        return 1\n",
            "    let lock_manifest_v2 = parsed_lock_v2 :: (std.manifest.LockManifestV2 :: metadata = empty_metadata, member_tables = empty_members, build_tables = empty_builds :: call) :: unwrap_or\n",
            "    let targets = lock_manifest_v2 :: \"path:game\" :: targets_for\n",
            "    arcana_process.io.print[Int] :: ((targets :: (std.collections.list.new[Str] :: :: call) :: unwrap_or) :: :: len) :: call\n",
            "    let package_ids = lock_manifest_v2 :: :: package_ids\n",
            "    arcana_process.io.print[Int] :: ((package_ids :: (std.collections.list.new[Str] :: :: call) :: unwrap_or) :: :: len) :: call\n",
            "    arcana_process.io.print[Str] :: ((lock_manifest_v2 :: :: workspace_root) :: \"missing\" :: unwrap_or) :: call\n",
            "    arcana_process.io.print[Str] :: ((lock_manifest_v2 :: \"path:game\", \"foo\" :: dep_for) :: \"missing\" :: unwrap_or) :: call\n",
            "    arcana_process.io.print[Str] :: ((lock_manifest_v2 :: \"path:game\", \"bar\" :: dep_for) :: \"missing\" :: unwrap_or) :: call\n",
            "    arcana_process.io.print[Str] :: ((lock_manifest_v2 :: \"registry:local:foo@1.2.3\" :: name_for) :: \"missing\" :: unwrap_or) :: call\n",
            "    arcana_process.io.print[Str] :: ((lock_manifest_v2 :: \"registry:local:foo@1.2.3\" :: source_kind_for) :: \"missing\" :: unwrap_or) :: call\n",
            "    arcana_process.io.print[Str] :: ((lock_manifest_v2 :: \"registry:local:foo@1.2.3\" :: version_for) :: \"missing\" :: unwrap_or) :: call\n",
            "    arcana_process.io.print[Str] :: ((lock_manifest_v2 :: \"registry:local:foo@1.2.3\" :: registry_for) :: \"missing\" :: unwrap_or) :: call\n",
            "    arcana_process.io.print[Str] :: ((lock_manifest_v2 :: \"registry:local:foo@1.2.3\" :: checksum_for) :: \"missing\" :: unwrap_or) :: call\n",
            "    arcana_process.io.print[Str] :: ((lock_manifest_v2 :: \"git:https://example.com/arcana/tooling.git#tag:v1.2.3:tooling\" :: source_kind_for) :: \"missing\" :: unwrap_or) :: call\n",
            "    arcana_process.io.print[Str] :: ((lock_manifest_v2 :: \"git:https://example.com/arcana/tooling.git#tag:v1.2.3:tooling\" :: git_for) :: \"missing\" :: unwrap_or) :: call\n",
            "    arcana_process.io.print[Str] :: ((lock_manifest_v2 :: \"git:https://example.com/arcana/tooling.git#tag:v1.2.3:tooling\" :: git_selector_for) :: \"missing\" :: unwrap_or) :: call\n",
            "    arcana_process.io.print[Str] :: ((lock_manifest_v2 :: \"path:game\" :: path_for) :: \"missing\" :: unwrap_or) :: call\n",
            "    arcana_process.io.print[Str] :: ((lock_manifest_v2 :: \"path:game\" :: kind_for) :: \"missing\" :: unwrap_or) :: call\n",
            "    let native_products = lock_manifest_v2 :: \"path:game\" :: native_product_names_for\n",
            "    arcana_process.io.print[Int] :: ((native_products :: (std.collections.list.new[Str] :: :: call) :: unwrap_or) :: :: len) :: call\n",
            "    arcana_process.io.print[Str] :: ((lock_manifest_v2 :: \"path:game\", \"default\" :: native_product_kind_for) :: \"missing\" :: unwrap_or) :: call\n",
            "    arcana_process.io.print[Str] :: ((lock_manifest_v2 :: \"path:game\", \"default\" :: native_product_role_for) :: \"missing\" :: unwrap_or) :: call\n",
            "    arcana_process.io.print[Str] :: ((lock_manifest_v2 :: \"path:game\", \"default\" :: native_product_producer_for) :: \"missing\" :: unwrap_or) :: call\n",
            "    arcana_process.io.print[Str] :: ((lock_manifest_v2 :: \"path:game\", \"default\" :: native_product_file_for) :: \"missing\" :: unwrap_or) :: call\n",
            "    arcana_process.io.print[Str] :: ((lock_manifest_v2 :: \"path:game\", \"default\" :: native_product_contract_for) :: \"missing\" :: unwrap_or) :: call\n",
            "    arcana_process.io.print[Str] :: ((lock_manifest_v2 :: \"path:game\", \"default\" :: native_product_rust_cdylib_crate_for) :: \"missing\" :: unwrap_or) :: call\n",
            "    let sidecars = lock_manifest_v2 :: \"path:game\", \"default\" :: native_product_sidecars_for\n",
            "    arcana_process.io.print[Int] :: ((sidecars :: (std.collections.list.new[Str] :: :: call) :: unwrap_or) :: :: len) :: call\n",
            "    arcana_process.io.print[Str] :: ((lock_manifest_v2 :: \"path:game\", \"internal-aot\" :: artifact_for) :: \"missing\" :: unwrap_or) :: call\n",
            "    arcana_process.io.print[Str] :: ((lock_manifest_v2 :: \"path:game\", \"internal-aot\" :: artifact_hash_for) :: \"missing\" :: unwrap_or) :: call\n",
            "    arcana_process.io.print[Str] :: ((lock_manifest_v2 :: \"path:game\", \"internal-aot\" :: format_for) :: \"missing\" :: unwrap_or) :: call\n",
            "    arcana_process.io.print[Str] :: ((lock_manifest_v2 :: \"path:game\", \"internal-aot\" :: toolchain_for) :: \"missing\" :: unwrap_or) :: call\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_std_manifest")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(
        host.stdout,
        vec![
            "2".to_string(),
            "0.1.0".to_string(),
            "registry".to_string(),
            "^1.2.3".to_string(),
            "local".to_string(),
            "../bar".to_string(),
            "2".to_string(),
            "grimoires/owned/app/game".to_string(),
            "app".to_string(),
            "arcana-aot-v2".to_string(),
            "path".to_string(),
            "arcana-aot-v2".to_string(),
            "path".to_string(),
            "cdylib".to_string(),
            "1".to_string(),
            "4".to_string(),
            "path:game".to_string(),
            "registry:local:foo@1.2.3".to_string(),
            "path:tools".to_string(),
            "foo".to_string(),
            "registry".to_string(),
            "1.2.3".to_string(),
            "local".to_string(),
            "sha256:abc123".to_string(),
            "git".to_string(),
            "https://example.com/arcana/tooling.git".to_string(),
            "tag:v1.2.3".to_string(),
            "grimoires/owned/app/game".to_string(),
            "app".to_string(),
            "1".to_string(),
            "cdylib".to_string(),
            "export".to_string(),
            "rust".to_string(),
            "game.dll".to_string(),
            "arcana-desktop-v1".to_string(),
            "arcana_game".to_string(),
            "2".to_string(),
            ".arcana/artifacts/game/internal-aot/app.artifact.toml".to_string(),
            "hash2".to_string(),
            "arcana-aot-v8".to_string(),
            "toolchain-1".to_string(),
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
            "import arcana_process.io\n",
            "fn main() -> Int:\n",
            "    let ch = std.concurrent.channel[Int] :: 2 :: call\n",
            "    ch :: 4 :: send\n",
            "    ch :: 9 :: send\n",
            "    arcana_process.io.print[Int] :: (ch :: :: recv) :: call\n",
            "    arcana_process.io.print[Int] :: (ch :: :: recv) :: call\n",
            "    let m = std.concurrent.mutex[Int] :: 11 :: call\n",
            "    arcana_process.io.print[Int] :: (m :: :: pull) :: call\n",
            "    m :: 15 :: put\n",
            "    arcana_process.io.print[Int] :: (m :: :: pull) :: call\n",
            "    let ai = std.concurrent.atomic_int :: 7 :: call\n",
            "    arcana_process.io.print[Int] :: (ai :: :: load) :: call\n",
            "    ai :: 5 :: add\n",
            "    ai :: 3 :: sub\n",
            "    arcana_process.io.print[Int] :: (ai :: :: load) :: call\n",
            "    arcana_process.io.print[Int] :: (ai :: 20 :: swap) :: call\n",
            "    arcana_process.io.print[Int] :: (ai :: :: load) :: call\n",
            "    let ab = std.concurrent.atomic_bool :: true :: call\n",
            "    arcana_process.io.print[Bool] :: (ab :: :: load) :: call\n",
            "    arcana_process.io.print[Bool] :: (ab :: false :: swap) :: call\n",
            "    arcana_process.io.print[Bool] :: (ab :: :: load) :: call\n",
            "    arcana_process.io.print[Int] :: (std.concurrent.thread_id :: :: call) :: call\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_std_concurrent")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
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
            "import arcana_process.io\n",
            "import std.memory\n",
            "record Item:\n",
            "    value: Int\n",
            "fn main() -> Int:\n",
            "    let mut arena_store = std.memory.new[Item] :: 4 :: call\n",
            "    let arena_id = arena: arena_store :> value = 7 <: Item\n",
            "    arcana_process.io.print[Int] :: (arena_store :: :: len) :: call\n",
            "    arcana_process.io.print[Bool] :: (arena_store :: arena_id :: has) :: call\n",
            "    let arena_item = arena_store :: arena_id :: get\n",
            "    arcana_process.io.print[Int] :: arena_item.value :: call\n",
            "    arena_store :: arena_id, (Item :: value = 9 :: call) :: set\n",
            "    let arena_item2 = arena_store :: arena_id :: get\n",
            "    arcana_process.io.print[Int] :: arena_item2.value :: call\n",
            "    arcana_process.io.print[Bool] :: (arena_store :: arena_id :: remove) :: call\n",
            "    arcana_process.io.print[Bool] :: (arena_store :: arena_id :: has) :: call\n",
            "    let mut frame_store = std.memory.frame_new[Item] :: 2 :: call\n",
            "    let frame_id = frame: frame_store :> value = 11 <: Item\n",
            "    let frame_item = frame_store :: frame_id :: get\n",
            "    arcana_process.io.print[Int] :: frame_item.value :: call\n",
            "    frame_store :: :: reset\n",
            "    arcana_process.io.print[Bool] :: (frame_store :: frame_id :: has) :: call\n",
            "    let mut pool_store = std.memory.pool_new[Item] :: 2 :: call\n",
            "    let pool_a = pool: pool_store :> value = 21 <: Item\n",
            "    let pool_item = pool_store :: pool_a :: get\n",
            "    arcana_process.io.print[Int] :: pool_item.value :: call\n",
            "    arcana_process.io.print[Bool] :: (pool_store :: pool_a :: remove) :: call\n",
            "    let pool_b = pool: pool_store :> value = 34 <: Item\n",
            "    arcana_process.io.print[Bool] :: (pool_store :: pool_a :: has) :: call\n",
            "    let pool_item2 = pool_store :: pool_b :: get\n",
            "    arcana_process.io.print[Int] :: pool_item2.value :: call\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_std_memory")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
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
            "import arcana_process.io\n",
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
            "    arcana_process.io.print[Int] :: current_value.value :: call\n",
            "    let mut slot = arena_store :: counter_id :: borrow_edit\n",
            "    bump :: slot :: call\n",
            "    let updated = arena_store :: counter_id :: get\n",
            "    arcana_process.io.print[Int] :: updated.value :: call\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_std_memory_borrow")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(host.stdout, vec!["9".to_string(), "10".to_string()]);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_memory_views_and_new_family_workspace() {
    let dir = temp_workspace_dir("memory_views_and_new_families");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_memory_views_new_families\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.binary\n",
            "import std.text\n",
            "import std.collections.array\n",
            "import std.collections.list\n",
            "import std.memory\n",
            "import std.text\n",
            "record Item:\n",
            "    value: Int\n",
            "fn main() -> Int:\n",
            "    let mut temp_store = std.memory.temp_new[Item] :: 2 :: call\n",
            "    let temp_a = temp: temp_store :> value = 3 <: Item\n",
            "    let _temp_b = temp: temp_store :> value = 4 <: Item\n",
            "    temp_store :: :: reset\n",
            "    if temp_store :: temp_a :: has:\n",
            "        return 101\n",
            "    let mut session_store = std.memory.session_new[Item] :: 2 :: call\n",
            "    let session_a = session: session_store :> value = 10 <: Item\n",
            "    let _session_b = session: session_store :> value = 12 <: Item\n",
            "    session_store :: :: seal\n",
            "    if not (session_store :: :: is_sealed):\n",
            "        return 102\n",
            "    let session_ids = session_store :: :: live_ids\n",
            "    if (session_ids :: :: len) != 2:\n",
            "        return 103\n",
            "    let mut session_live = std.collections.list.new[Int] :: :: call\n",
            "    for id in session_ids:\n",
            "        session_live :: (session_store :: id :: get).value :: push\n",
            "    let session_order = std.collections.array.from_list[Int] :: session_live :: call\n",
            "    if session_order[0] != 10:\n",
            "        return 127\n",
            "    if session_order[1] != 12:\n",
            "        return 128\n",
            "    session_store :: :: unseal\n",
            "    session_store :: session_a, (Item :: value = 11 :: call) :: set\n",
            "    if (session_store :: session_a :: get).value != 11:\n",
            "        return 104\n",
            "    let mut ring_store = std.memory.ring_new[Item] :: 2 :: call\n",
            "    let ring_a = ring: ring_store :> value = 20 <: Item\n",
            "    let ring_b = ring: ring_store :> value = 21 <: Item\n",
            "    let ring_c = ring: ring_store :> value = 22 <: Item\n",
            "    if ring_store :: ring_a :: has:\n",
            "        return 105\n",
            "    if not (ring_store :: ring_b :: has):\n",
            "        return 106\n",
            "    if not (ring_store :: ring_c :: has):\n",
            "        return 107\n",
            "    if true:\n",
            "        let ring_window = ring_store :: 0, 2 :: window_read\n",
            "        if (ring_window :: 0 :: get).value != 21:\n",
            "            return 108\n",
            "        if (ring_window :: 1 :: get).value != 22:\n",
            "            return 109\n",
            "        ring_store :: ring_b, (Item :: value = 23 :: call) :: set\n",
            "        if (ring_window :: 0 :: get).value != 23:\n",
            "            return 137\n",
            "    if true:\n",
            "        let mut ring_window_edit = ring_store :: 0, 2 :: window_edit\n",
            "        ring_window_edit :: 1, (Item :: value = 24 :: call) :: set\n",
            "    if (ring_store :: ring_c :: get).value != 24:\n",
            "        return 138\n",
            "    let mut slab_store = std.memory.slab_new[Item] :: 2 :: call\n",
            "    let slab_a = slab: slab_store :> value = 30 <: Item\n",
            "    let _slab_b = slab: slab_store :> value = 31 <: Item\n",
            "    if not (slab_store :: slab_a :: remove):\n",
            "        return 110\n",
            "    let slab_c = slab: slab_store :> value = 32 <: Item\n",
            "    if slab_store :: slab_a :: has:\n",
            "        return 111\n",
            "    if (slab_store :: slab_c :: get).value != 32:\n",
            "        return 112\n",
            "    let slab_ids = slab_store :: :: live_ids\n",
            "    let mut slab_live = std.collections.list.new[Int] :: :: call\n",
            "    for id in slab_ids:\n",
            "        slab_live :: (slab_store :: id :: get).value :: push\n",
            "    let slab_order = std.collections.array.from_list[Int] :: slab_live :: call\n",
            "    if slab_order[0] != 32:\n",
            "        return 129\n",
            "    if slab_order[1] != 31:\n",
            "        return 130\n",
            "    slab_store :: :: seal\n",
            "    if not (slab_store :: :: is_sealed):\n",
            "        return 113\n",
            "    slab_store :: :: unseal\n",
            "    let mut read_values = std.collections.array.new[Int] :: 4, 0 :: call\n",
            "    read_values[0] = 18\n",
            "    read_values[1] = 52\n",
            "    read_values[2] = 171\n",
            "    read_values[3] = 205\n",
            "    let view = &read read_values[1..3]\n",
            "    if (view :: :: len) != 2:\n",
            "        return 114\n",
            "    if (view :: 0 :: get) != 52:\n",
            "        return 115\n",
            "    let mut edit_values = std.collections.array.new[Int] :: 2, 0 :: call\n",
            "    edit_values[0] = 18\n",
            "    edit_values[1] = 52\n",
            "    if true:\n",
            "        let mut edit = &edit edit_values[0..2]\n",
            "        edit :: 1, 255 :: set\n",
            "        if (edit :: 1 :: get) != 255:\n",
            "            return 116\n",
            "    if edit_values[1] != 255:\n",
            "        return 121\n",
            "    let mut byte_values = std.collections.array.new[Int] :: 4, 0 :: call\n",
            "    byte_values[0] = 18\n",
            "    byte_values[1] = 52\n",
            "    byte_values[2] = 171\n",
            "    byte_values[3] = 205\n",
            "    let bytes = byte_values[1..3]\n",
            "    if ((bytes :: 0 :: get) * 256 + (bytes :: 1 :: get)) != 13483:\n",
            "        return 117\n",
            "    if true:\n",
            "        let mut bytes_edit = &edit byte_values[0..2]\n",
            "        bytes_edit :: 1, 99 :: set\n",
            "    if byte_values[1] != 99:\n",
            "        return 133\n",
            "    let bytes_copy_view = byte_values[1..3]\n",
            "    let bytes_copy = bytes_copy_view :: :: to_array\n",
            "    if bytes_copy[0] != 99:\n",
            "        return 134\n",
            "    if bytes_copy[1] != 171:\n",
            "        return 135\n",
            "    let mut writer = std.binary.writer :: :: call\n",
            "    writer :: 4660 :: push_u16_be\n",
            "    let written = writer :: :: into_bytes\n",
            "    if (std.text.bytes_at :: written, 0 :: call) != 18:\n",
            "        return 131\n",
            "    if (std.text.bytes_at :: written, 1 :: call) != 52:\n",
            "        return 132\n",
            "    let text = \"hello\"\n",
            "    let text_view = &read text[1..4]\n",
            "    if (text_view :: :: len) != 3:\n",
            "        return 118\n",
            "    if (text_view :: 0 :: get) != (U8 :: 101 :: call):\n",
            "        return 119\n",
            "    if (text_view :: :: to_str) != \"ell\":\n",
            "        return 120\n",
            "    let copied_text_view = text[1..4]\n",
            "    let text_copy = copied_text_view :: :: to_str\n",
            "    if text_copy != \"ell\":\n",
            "        return 136\n",
            "    return 0\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// memory test types\n");

    let graph = load_workspace_graph(&dir).expect("workspace graph should load");
    let checked = check_workspace_graph(&graph).expect("workspace should check");
    let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
        .expect("fingerprints should compute");
    let order = plan_workspace(&graph).expect("workspace order should plan");
    let statuses =
        plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_memory_views_new_families")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn workspace_checks_memory_and_binary_trait_surface() {
    let dir = temp_workspace_dir("memory_binary_trait_surface");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_memory_binary_trait_surface\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.binary\n",
            "import std.collections.array\n",
            "import std.memory\n",
            "import std.option\n",
            "use std.binary.BinaryReadable\n",
            "use std.binary.ByteSink\n",
            "use std.memory.Compactable\n",
            "",
            "use std.memory.IdAllocating\n",
            "use std.memory.LiveIterable\n",
            "use std.memory.Resettable\n",
            "use std.memory.Sealable\n",
            "use std.memory.SequenceBuffer\n",
            "",
            "record Item:\n",
            "    value: Int\n",
            "record Header:\n",
            "    value: Int\n",
            "impl std.binary.BinaryReadable[Header] for Header:\n",
            "    fn read_from(edit reader: std.binary.Reader) -> Header:\n",
            "        return Header :: value = (reader :: :: read_u16_be) :: call\n",
            "impl std.binary.ByteSink[Header] for Header:\n",
            "    fn write_to(read value: Header, edit writer: std.binary.Writer):\n",
            "        writer :: value.value :: push_u16_be\n",
            "fn clear_store[S, where std.memory.Resettable[S]](edit source: S):\n",
            "    source :: :: reset_value\n",
            "fn first_value(read source: Array[Int]) -> Int:\n",
            "    return source[0]\n",
            "fn overwrite_head(edit source: Array[Int], value: Int) -> Int:\n",
            "    source[0] = value\n",
            "    return source[0]\n",
            "fn supports_id[S, where std.memory.IdAllocating[S]](read source: S, id: std.memory.IdAllocating[S].Id) -> Bool:\n",
            "    return source :: id :: has_id\n",
            "fn live_count[S, where std.memory.LiveIterable[S]](read source: S) -> Int:\n",
            "    return (source :: :: live_ids_of) :: :: len\n",
            "fn publish_cycle[S, where std.memory.Sealable[S]](edit source: S) -> Bool:\n",
            "    source :: :: seal_state\n",
            "    let sealed = source :: :: state_is_sealed\n",
            "    source :: :: unseal_state\n",
            "    return sealed and (not (source :: :: state_is_sealed))\n",
            "fn compact_count[S, where std.memory.Compactable[S]](edit source: S) -> Int:\n",
            "    return (source :: :: compact_items) :: :: len\n",
            "fn push_pop[S, where std.memory.SequenceBuffer[S]](edit source: S, take value: std.memory.SequenceBuffer[S].Item) -> Bool:\n",
            "    let _ = source :: value :: push_item\n",
            "    let popped = source :: :: pop_item\n",
            "    return match popped:\n",
            "        Option.Some(_) => true\n",
            "        Option.None => false\n",
            "fn main() -> Int:\n",
            "    return 0\n",
        ),
    );
    write_file(
        &dir.join("src").join("types.arc"),
        "// trait surface types\n",
    );

    let graph = load_workspace_graph(&dir).expect("workspace graph should load");
    let _checked = check_workspace_graph(&graph).expect("workspace should check");

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_memory_and_binary_trait_helpers_workspace() {
    let dir = temp_workspace_dir("memory_binary_trait_helpers");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_memory_binary_trait_helpers\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.binary\n",
            "import std.option\n",
            "import std.memory\n",
            "use std.binary.ByteSink\n",
            "use std.memory.Compactable\n",
            "",
            "use std.memory.IdAllocating\n",
            "use std.memory.LiveIterable\n",
            "use std.memory.Resettable\n",
            "use std.memory.Sealable\n",
            "use std.memory.SequenceBuffer\n",
            "",
            "record Item:\n",
            "    value: Int\n",
            "record Header:\n",
            "    value: Int\n",
            "impl std.binary.ByteSink[Header] for Header:\n",
            "    fn write_to(read value: Header, edit writer: std.binary.Writer):\n",
            "        writer :: value.value :: push_u16_be\n",
            "fn clear_store[S, where std.memory.Resettable[S]](edit source: S):\n",
            "    source :: :: reset_value\n",
            "fn first_value(read source: Array[Int]) -> Int:\n",
            "    return source[0]\n",
            "fn overwrite_head(edit source: Array[Int], value: Int) -> Int:\n",
            "    source[0] = value\n",
            "    return source[0]\n",
            "fn supports_id[S, where std.memory.IdAllocating[S]](read source: S, id: std.memory.IdAllocating[S].Id) -> Bool:\n",
            "    return source :: id :: has_id\n",
            "fn live_count[S, where std.memory.LiveIterable[S]](read source: S) -> Int:\n",
            "    return (source :: :: live_ids_of) :: :: len\n",
            "fn publish_cycle[S, where std.memory.Sealable[S]](edit source: S) -> Bool:\n",
            "    source :: :: seal_state\n",
            "    let sealed = source :: :: state_is_sealed\n",
            "    source :: :: unseal_state\n",
            "    return sealed and (not (source :: :: state_is_sealed))\n",
            "fn compact_count[S, where std.memory.Compactable[S]](edit source: S) -> Int:\n",
            "    return (source :: :: compact_items) :: :: len\n",
            "fn push_pop[S, where std.memory.SequenceBuffer[S]](edit source: S, take value: std.memory.SequenceBuffer[S].Item) -> Bool:\n",
            "    let _ = source :: value :: push_item\n",
            "    let popped = source :: :: pop_item\n",
            "    return match popped:\n",
            "        Option.Some(_) => true\n",
            "        Option.None => false\n",
            "fn emit_word[S, where std.binary.ByteSink[S]](read value: S) -> Int:\n",
            "    let mut writer = std.binary.writer :: :: call\n",
            "    value :: writer :: write_to\n",
            "    let bytes = writer :: :: into_bytes\n",
            "    return ((std.text.bytes_at :: bytes, 0 :: call) << 8) | (std.text.bytes_at :: bytes, 1 :: call)\n",
            "fn main() -> Int:\n",
            "    let mut temp_store = std.memory.temp_new[Item] :: 2 :: call\n",
            "    let temp_id = temp: temp_store :> value = 1 <: Item\n",
            "    clear_store :: temp_store :: call\n",
            "    if temp_store :: temp_id :: has:\n",
            "        return 201\n",
            "    let mut values = std.collections.array.new[Int] :: 2, 0 :: call\n",
            "    values[0] = 7\n",
            "    values[1] = 8\n",
            "    if (first_value :: values :: call) != 7:\n",
            "        return 202\n",
            "    if (overwrite_head :: values, 9 :: call) != 9:\n",
            "        return 203\n",
            "    if values[0] != 9:\n",
            "        return 204\n",
            "    let mut session_store = std.memory.session_new[Item] :: 2 :: call\n",
            "    let session_id = session: session_store :> value = 11 <: Item\n",
            "    if not (supports_id :: session_store, session_id :: call):\n",
            "        return 205\n",
            "    if (live_count :: session_store :: call) != 1:\n",
            "        return 206\n",
            "    if not (publish_cycle :: session_store :: call):\n",
            "        return 207\n",
            "    let mut pool_store = std.memory.pool_new[Item] :: 2 :: call\n",
            "    let pool_a = pool: pool_store :> value = 21 <: Item\n",
            "    let _pool_b = pool: pool_store :> value = 22 <: Item\n",
            "    if not (pool_store :: pool_a :: remove):\n",
            "        return 208\n",
            "    if (compact_count :: pool_store :: call) != 1:\n",
            "        return 209\n",
            "    let mut ring_store = std.memory.ring_new[Item] :: 2 :: call\n",
            "    if not (push_pop :: ring_store, (Item :: value = 30 :: call) :: call):\n",
            "        return 210\n",
            "    let header = Header :: value = 4660 :: call\n",
            "    if (emit_word :: header :: call) != 4660:\n",
            "        return 211\n",
            "    return 0\n",
        ),
    );
    write_file(
        &dir.join("src").join("types.arc"),
        "// trait execution types\n",
    );

    let plan = build_workspace_plan_for_member(&dir, "runtime_memory_binary_trait_helpers");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute trait helpers");

    assert_eq!(code, 0);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_resolves_overloaded_method_on_borrowed_receiver() {
    let dir = temp_workspace_dir("borrowed_receiver_method");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_borrowed_receiver_method\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import arcana_process.io\n",
            "import std.memory\n",
            "record Counter:\n",
            "    value: Int\n",
            "record Gauge:\n",
            "    value: Int\n",
            "impl Counter:\n",
            "    fn bump(edit self: Counter) -> Int:\n",
            "        self.value += 1\n",
            "        return self.value\n",
            "impl Gauge:\n",
            "    fn bump(read self: Gauge) -> Int:\n",
            "        return self.value + 100\n",
            "fn main() -> Int:\n",
            "    let mut arena_store = std.memory.new[Counter] :: 1 :: call\n",
            "    let counter_id = arena: arena_store :> value = 9 <: Counter\n",
            "    let mut slot = arena_store :: counter_id :: borrow_edit\n",
            "    let bumped = slot :: :: bump\n",
            "    arcana_process.io.print[Int] :: bumped :: call\n",
            "    let updated = arena_store :: counter_id :: get\n",
            "    arcana_process.io.print[Int] :: updated.value :: call\n",
            "    return 0\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let plan = build_workspace_plan_for_member(&dir, "runtime_borrowed_receiver_method");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should dispatch borrowed receiver");

    assert_eq!(code, 0);
    assert_eq!(host.stdout, vec!["10".to_string(), "10".to_string()]);

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
            "import arcana_process.io\n",
            "import std.memory\n",
            "record Counter:\n",
            "    value: Int\n",
            "fn make_counter(value: Int, bonus: Int) -> Counter:\n",
            "    arcana_process.io.print[Int] :: bonus :: call\n",
            "    return Counter :: value = value + bonus :: call\n",
            "fn main() -> Int:\n",
            "    let mut arena_store = std.memory.new[Counter] :: 2 :: call\n",
            "    arena: arena_store :> 9 <: make_counter\n",
            "        bonus = 4\n",
            "    arcana_process.io.print[Int] :: (arena_store :: :: len) :: call\n",
            "    let id = arena: arena_store :> value = 1 <: Counter\n",
            "    let item = arena_store :: id :: get\n",
            "    arcana_process.io.print[Int] :: item.value :: call\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_memory_phrase_attachments")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
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
fn execute_main_resolves_module_and_block_memory_specs() {
    let dir = temp_workspace_dir("headed_region_memory_specs");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_headed_region_memory_specs\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import types\n",
            "fn main() -> Int:\n",
            "    Memory pool:scratch -alloc\n",
            "        capacity = 2\n",
            "        pressure = bounded\n",
            "    let _a = arena: types.cache :> value = 7 <: types.Item\n",
            "    let _b = pool: scratch :> value = 9 <: types.Item\n",
            "    let _c = arena: types.cache :> value = 11 <: types.Item\n",
            "    return 3\n",
        ),
    );
    write_file(
        &dir.join("src").join("types.arc"),
        concat!(
            "export record Item:\n",
            "    value: Int\n",
            "Memory arena:cache -alloc\n",
            "    capacity = 4\n",
            "    pressure = bounded\n",
        ),
    );

    let graph = load_workspace_graph(&dir).expect("workspace graph should load");
    let checked = check_workspace_graph(&graph).expect("workspace should check");
    let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
        .expect("fingerprints should compute");
    let order = plan_workspace(&graph).expect("workspace order should plan");
    let statuses =
        plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_headed_region_memory_specs")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 3);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_memory_specs_apply_runtime_policies() {
    let dir = temp_workspace_dir("headed_region_memory_policies");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_headed_region_memory_policies\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import types\n",
            "fn main() -> Int:\n",
            "    let _stable_a = arena: types.stable_cache :> value = 1 <: types.Item\n",
            "    let _stable_b = arena: types.stable_cache :> value = 2 <: types.Item\n",
            "    let _fresh_a = arena: types.fresh_cache :> value = 3 <: types.Item\n",
            "    let _fresh_b = arena: types.fresh_cache :> value = 4 <: types.Item\n",
            "    let _frame_a = frame: types.scratch :> value = 5 <: types.Item\n",
            "    let _frame_b = frame: types.scratch :> value = 6 <: types.Item\n",
            "    let _free_pool = pool: types.free_pool :> value = 7 <: types.Item\n",
            "    let _strict_pool = pool: types.strict_pool :> value = 8 <: types.Item\n",
            "    return 0\n",
        ),
    );
    write_file(
        &dir.join("src").join("types.arc"),
        concat!(
            "export record Item:\n",
            "    value: Int\n",
            "Memory arena:stable_cache -grow\n",
            "    capacity = 1\n",
            "    growth = 2\n",
            "    pressure = elastic\n",
            "    handle = stable\n",
            "Memory arena:fresh_cache -grow\n",
            "    capacity = 1\n",
            "    growth = 1\n",
            "    pressure = elastic\n",
            "    handle = unstable\n",
            "Memory frame:scratch -recycle\n",
            "    capacity = 1\n",
            "    recycle = frame\n",
            "Memory pool:free_pool -recycle\n",
            "    capacity = 3\n",
            "    recycle = free_list\n",
            "Memory pool:strict_pool -alloc\n",
            "    capacity = 3\n",
            "    recycle = strict\n",
        ),
    );

    let plan = build_workspace_plan_for_member(&dir, "runtime_headed_region_memory_policies");
    let mut host = BufferedHost::default();
    let mut state = RuntimeExecutionState::default();
    let entry = plan
        .main_entrypoint()
        .expect("main entrypoint should exist");
    let value = execute_routine_with_state(
        &plan,
        entry.routine_index,
        Vec::new(),
        Vec::new(),
        &mut state,
        &mut host,
    )
    .expect("runtime should execute");
    assert_eq!(value, RuntimeValue::Int(0));

    let stable_spec = state
        .module_memory_specs
        .values()
        .find(|spec| spec.spec.name == "stable_cache")
        .expect("stable cache spec should materialize");
    let RuntimeValue::Opaque(RuntimeOpaqueValue::Arena(stable_handle)) = stable_spec
        .handle
        .clone()
        .expect("stable cache should retain a cached handle")
    else {
        panic!("stable cache should cache an arena handle");
    };
    let stable_arena = state
        .arenas
        .get(&stable_handle)
        .expect("stable arena should exist");
    assert_eq!(stable_arena.slots.len(), 2);
    assert_eq!(stable_arena.policy.current_limit, 3);

    let fresh_spec = state
        .module_memory_specs
        .values()
        .find(|spec| spec.spec.name == "fresh_cache")
        .expect("fresh cache spec should materialize");
    assert_eq!(fresh_spec.handle, None);
    assert_eq!(state.arenas.len(), 3);

    let frame_spec = state
        .module_memory_specs
        .values()
        .find(|spec| spec.spec.name == "scratch")
        .expect("scratch frame spec should materialize");
    let RuntimeValue::Opaque(RuntimeOpaqueValue::FrameArena(frame_handle)) = frame_spec
        .handle
        .clone()
        .expect("frame spec should keep a cached handle")
    else {
        panic!("frame spec should cache a frame handle");
    };
    let frame_arena = state
        .frame_arenas
        .get(&frame_handle)
        .expect("frame arena should exist");
    assert_eq!(frame_arena.slots.len(), 1);
    assert_eq!(frame_arena.generation, 1);

    let free_spec = state
        .module_memory_specs
        .values()
        .find(|spec| spec.spec.name == "free_pool")
        .expect("free pool spec should materialize");
    let RuntimeValue::Opaque(RuntimeOpaqueValue::PoolArena(free_handle)) = free_spec
        .handle
        .clone()
        .expect("free pool should keep a cached handle")
    else {
        panic!("free pool should cache a pool handle");
    };
    let strict_spec = state
        .module_memory_specs
        .values()
        .find(|spec| spec.spec.name == "strict_pool")
        .expect("strict pool spec should materialize");
    let RuntimeValue::Opaque(RuntimeOpaqueValue::PoolArena(strict_handle)) = strict_spec
        .handle
        .clone()
        .expect("strict pool should keep a cached handle")
    else {
        panic!("strict pool should cache a pool handle");
    };

    let free_id = execute_runtime_intrinsic(
        RuntimeIntrinsic::MemoryPoolAlloc,
        &[],
        &mut vec![
            RuntimeValue::Opaque(RuntimeOpaqueValue::PoolArena(free_handle)),
            RuntimeValue::Int(11),
        ],
        &plan,
        None,
        None,
        None,
        None,
        None,
        &mut state,
        &mut host,
    )
    .expect("free-list pool should allocate");
    execute_runtime_intrinsic(
        RuntimeIntrinsic::MemoryPoolRemove,
        &[],
        &mut vec![
            RuntimeValue::Opaque(RuntimeOpaqueValue::PoolArena(free_handle)),
            free_id.clone(),
        ],
        &plan,
        None,
        None,
        None,
        None,
        None,
        &mut state,
        &mut host,
    )
    .expect("free-list pool should remove");
    let _ = execute_runtime_intrinsic(
        RuntimeIntrinsic::MemoryPoolAlloc,
        &[],
        &mut vec![
            RuntimeValue::Opaque(RuntimeOpaqueValue::PoolArena(free_handle)),
            RuntimeValue::Int(12),
        ],
        &plan,
        None,
        None,
        None,
        None,
        None,
        &mut state,
        &mut host,
    )
    .expect("free-list pool should recycle a freed slot");

    let strict_id = execute_runtime_intrinsic(
        RuntimeIntrinsic::MemoryPoolAlloc,
        &[],
        &mut vec![
            RuntimeValue::Opaque(RuntimeOpaqueValue::PoolArena(strict_handle)),
            RuntimeValue::Int(21),
        ],
        &plan,
        None,
        None,
        None,
        None,
        None,
        &mut state,
        &mut host,
    )
    .expect("strict pool should allocate");
    execute_runtime_intrinsic(
        RuntimeIntrinsic::MemoryPoolRemove,
        &[],
        &mut vec![
            RuntimeValue::Opaque(RuntimeOpaqueValue::PoolArena(strict_handle)),
            strict_id.clone(),
        ],
        &plan,
        None,
        None,
        None,
        None,
        None,
        &mut state,
        &mut host,
    )
    .expect("strict pool should remove");
    let _ = execute_runtime_intrinsic(
        RuntimeIntrinsic::MemoryPoolAlloc,
        &[],
        &mut vec![
            RuntimeValue::Opaque(RuntimeOpaqueValue::PoolArena(strict_handle)),
            RuntimeValue::Int(22),
        ],
        &plan,
        None,
        None,
        None,
        None,
        None,
        &mut state,
        &mut host,
    )
    .expect("strict pool should allocate a fresh slot");
    execute_runtime_intrinsic(
        RuntimeIntrinsic::MemoryPoolReset,
        &[],
        &mut vec![RuntimeValue::Opaque(RuntimeOpaqueValue::PoolArena(
            strict_handle,
        ))],
        &plan,
        None,
        None,
        None,
        None,
        None,
        &mut state,
        &mut host,
    )
    .expect("strict pool should reset");
    let strict_reused = execute_runtime_intrinsic(
        RuntimeIntrinsic::MemoryPoolAlloc,
        &[],
        &mut vec![
            RuntimeValue::Opaque(RuntimeOpaqueValue::PoolArena(strict_handle)),
            RuntimeValue::Int(23),
        ],
        &plan,
        None,
        None,
        None,
        None,
        None,
        &mut state,
        &mut host,
    )
    .expect("strict pool should reuse slots after reset");
    let RuntimeValue::Opaque(RuntimeOpaqueValue::PoolId(strict_reused_id)) = strict_reused else {
        panic!("strict pool alloc should return a PoolId");
    };
    assert_eq!(strict_reused_id.slot, 0);

    let free_pool = state
        .pool_arenas
        .get(&free_handle)
        .expect("free-list pool should exist");
    assert_eq!(free_pool.next_slot, 2);
    assert!(free_pool.free_slots.is_empty());

    let strict_pool = state
        .pool_arenas
        .get(&strict_handle)
        .expect("strict pool should exist");
    assert_eq!(strict_pool.next_slot, 1);
    assert!(strict_pool.free_slots.is_empty());

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn runtime_memory_spec_materialization_uses_descriptor_hook_registry() {
    let mut state = RuntimeExecutionState::default();
    let arena_descriptor =
        arcana_language_law::memory_family_descriptor(arcana_language_law::MemoryFamily::Arena);
    let frame_descriptor =
        arcana_language_law::memory_family_descriptor(arcana_language_law::MemoryFamily::Frame);
    let pool_descriptor =
        arcana_language_law::memory_family_descriptor(arcana_language_law::MemoryFamily::Pool);

    let arena_value = super::materialize_runtime_memory_spec_handle(
        &super::RuntimeMemorySpecMaterialization {
            hook_id: arena_descriptor.lazy_materialization_hook_id,
            handle_policy: super::RuntimeMemoryHandlePolicy::Stable,
            kind: super::RuntimeMemorySpecMaterializationKind::Arena(
                super::default_runtime_arena_policy(2),
            ),
        },
        &mut state,
    )
    .expect("arena descriptor hook should materialize");
    let frame_value = super::materialize_runtime_memory_spec_handle(
        &super::RuntimeMemorySpecMaterialization {
            hook_id: frame_descriptor.lazy_materialization_hook_id,
            handle_policy: super::RuntimeMemoryHandlePolicy::Stable,
            kind: super::RuntimeMemorySpecMaterializationKind::Frame(
                super::default_runtime_frame_policy(2),
            ),
        },
        &mut state,
    )
    .expect("frame descriptor hook should materialize");
    let pool_value = super::materialize_runtime_memory_spec_handle(
        &super::RuntimeMemorySpecMaterialization {
            hook_id: pool_descriptor.lazy_materialization_hook_id,
            handle_policy: super::RuntimeMemoryHandlePolicy::Stable,
            kind: super::RuntimeMemorySpecMaterializationKind::Pool(
                super::default_runtime_pool_policy(2),
            ),
        },
        &mut state,
    )
    .expect("pool descriptor hook should materialize");

    assert!(matches!(
        arena_value,
        RuntimeValue::Opaque(RuntimeOpaqueValue::Arena(_))
    ));
    assert!(matches!(
        frame_value,
        RuntimeValue::Opaque(RuntimeOpaqueValue::FrameArena(_))
    ));
    assert!(matches!(
        pool_value,
        RuntimeValue::Opaque(RuntimeOpaqueValue::PoolArena(_))
    ));
    assert_eq!(state.arenas.len(), 1);
    assert_eq!(state.frame_arenas.len(), 1);
    assert_eq!(state.pool_arenas.len(), 1);

    let err = super::materialize_runtime_memory_spec_handle(
        &super::RuntimeMemorySpecMaterialization {
            hook_id: "unknown_memory_new",
            handle_policy: super::RuntimeMemoryHandlePolicy::Stable,
            kind: super::RuntimeMemorySpecMaterializationKind::Arena(
                super::default_runtime_arena_policy(1),
            ),
        },
        &mut state,
    )
    .expect_err("unknown runtime memory hooks should fail");
    assert!(
        super::runtime_eval_message(err)
            .contains("unsupported runtime memory materialization hook `unknown_memory_new`")
    );
}

#[test]
fn session_unseal_rejects_live_reference_views() {
    let plan = empty_runtime_plan("demo");
    let mut state = RuntimeExecutionState::default();
    let handle = insert_runtime_session_arena(
        &mut state,
        &["Int".to_string()],
        default_runtime_session_policy(4),
    );
    let id = RuntimeSessionIdValue {
        arena: handle,
        slot: 0,
        generation: 0,
    };
    let arena = state
        .session_arenas
        .get_mut(&handle)
        .expect("session arena should exist");
    arena
        .slots
        .insert(0, RuntimeValue::Array(vec![RuntimeValue::Int(1)]));
    arena.next_slot = 1;
    arena.sealed = true;
    let _ = insert_runtime_read_view_from_reference(
        &mut state,
        &["Int".to_string()],
        RuntimeReferenceValue {
            mode: RuntimeReferenceMode::Read,
            target: RuntimeReferenceTarget::SessionSlot {
                id,
                members: Vec::new(),
            },
        },
        0,
        1,
    );

    let mut host = BufferedHost::default();
    let err = execute_runtime_intrinsic(
        RuntimeIntrinsic::MemorySessionUnseal,
        &[],
        &mut vec![RuntimeValue::Opaque(RuntimeOpaqueValue::SessionArena(
            handle,
        ))],
        &plan,
        None,
        None,
        None,
        None,
        None,
        &mut state,
        &mut host,
    )
    .expect_err("session unseal should reject while borrowed views are live");
    assert!(err.contains("borrowed views"), "{err}");
}

#[test]
fn session_borrow_edit_rejects_while_sealed() {
    let plan = empty_runtime_plan("demo");
    let mut state = RuntimeExecutionState::default();
    let handle = insert_runtime_session_arena(
        &mut state,
        &["Int".to_string()],
        default_runtime_session_policy(2),
    );
    let id = RuntimeSessionIdValue {
        arena: handle,
        slot: 0,
        generation: 0,
    };
    let arena = state
        .session_arenas
        .get_mut(&handle)
        .expect("session arena should exist");
    arena.slots.insert(0, RuntimeValue::Int(7));
    arena.next_slot = 1;
    arena.sealed = true;

    let mut host = BufferedHost::default();
    let err = execute_runtime_intrinsic(
        RuntimeIntrinsic::MemorySessionBorrowEdit,
        &[],
        &mut vec![
            RuntimeValue::Opaque(RuntimeOpaqueValue::SessionArena(handle)),
            RuntimeValue::Opaque(RuntimeOpaqueValue::SessionId(id)),
        ],
        &plan,
        None,
        None,
        None,
        None,
        None,
        &mut state,
        &mut host,
    )
    .expect_err("session borrow_edit should reject while sealed");
    assert!(err.contains("while sealed"), "{err}");
}

#[test]
fn session_unseal_rejects_live_borrows() {
    let plan = empty_runtime_plan("demo");
    let mut state = RuntimeExecutionState::default();
    let handle = insert_runtime_session_arena(
        &mut state,
        &["Int".to_string()],
        default_runtime_session_policy(2),
    );
    let id = RuntimeSessionIdValue {
        arena: handle,
        slot: 0,
        generation: 0,
    };
    let arena = state
        .session_arenas
        .get_mut(&handle)
        .expect("session arena should exist");
    arena.slots.insert(0, RuntimeValue::Int(3));
    arena.next_slot = 1;
    arena.sealed = true;

    let mut scopes = vec![RuntimeScope::default()];
    insert_runtime_local(
        &mut state,
        0,
        &mut scopes[0],
        0,
        "borrow".to_string(),
        false,
        RuntimeValue::Ref(RuntimeReferenceValue {
            mode: RuntimeReferenceMode::Read,
            target: RuntimeReferenceTarget::SessionSlot {
                id,
                members: Vec::new(),
            },
        }),
    );

    let mut host = BufferedHost::default();
    let err = execute_runtime_intrinsic(
        RuntimeIntrinsic::MemorySessionUnseal,
        &[],
        &mut vec![RuntimeValue::Opaque(RuntimeOpaqueValue::SessionArena(
            handle,
        ))],
        &plan,
        Some(&mut scopes),
        None,
        None,
        None,
        None,
        &mut state,
        &mut host,
    )
    .expect_err("session unseal should reject while borrows are live");
    assert!(err.contains("borrows"), "{err}");
}

#[test]
fn slab_unseal_rejects_live_reference_views() {
    let plan = empty_runtime_plan("demo");
    let mut state = RuntimeExecutionState::default();
    let handle = insert_runtime_slab(
        &mut state,
        &["Int".to_string()],
        default_runtime_slab_policy(4),
    );
    let id = RuntimeSlabIdValue {
        arena: handle,
        slot: 0,
        generation: 0,
    };
    let arena = state.slabs.get_mut(&handle).expect("slab should exist");
    arena
        .slots
        .insert(0, RuntimeValue::Array(vec![RuntimeValue::Int(1)]));
    arena.generations.insert(0, 0);
    arena.next_slot = 1;
    arena.sealed = true;
    let _ = insert_runtime_read_view_from_reference(
        &mut state,
        &["Int".to_string()],
        RuntimeReferenceValue {
            mode: RuntimeReferenceMode::Read,
            target: RuntimeReferenceTarget::SlabSlot {
                id,
                members: Vec::new(),
            },
        },
        0,
        1,
    );

    let mut host = BufferedHost::default();
    let err = execute_runtime_intrinsic(
        RuntimeIntrinsic::MemorySlabUnseal,
        &[],
        &mut vec![RuntimeValue::Opaque(RuntimeOpaqueValue::Slab(handle))],
        &plan,
        None,
        None,
        None,
        None,
        None,
        &mut state,
        &mut host,
    )
    .expect_err("slab unseal should reject while borrowed views are live");
    assert!(err.contains("borrowed views"), "{err}");
}

#[test]
fn slab_borrow_edit_rejects_while_sealed() {
    let plan = empty_runtime_plan("demo");
    let mut state = RuntimeExecutionState::default();
    let handle = insert_runtime_slab(
        &mut state,
        &["Int".to_string()],
        default_runtime_slab_policy(2),
    );
    let id = RuntimeSlabIdValue {
        arena: handle,
        slot: 0,
        generation: 0,
    };
    let arena = state.slabs.get_mut(&handle).expect("slab should exist");
    arena.slots.insert(0, RuntimeValue::Int(9));
    arena.generations.insert(0, 0);
    arena.next_slot = 1;
    arena.sealed = true;

    let mut host = BufferedHost::default();
    let err = execute_runtime_intrinsic(
        RuntimeIntrinsic::MemorySlabBorrowEdit,
        &[],
        &mut vec![
            RuntimeValue::Opaque(RuntimeOpaqueValue::Slab(handle)),
            RuntimeValue::Opaque(RuntimeOpaqueValue::SlabId(id)),
        ],
        &plan,
        None,
        None,
        None,
        None,
        None,
        &mut state,
        &mut host,
    )
    .expect_err("slab borrow_edit should reject while sealed");
    assert!(err.contains("while sealed"), "{err}");
}

#[test]
fn slab_unseal_rejects_live_borrows() {
    let plan = empty_runtime_plan("demo");
    let mut state = RuntimeExecutionState::default();
    let handle = insert_runtime_slab(
        &mut state,
        &["Int".to_string()],
        default_runtime_slab_policy(2),
    );
    let id = RuntimeSlabIdValue {
        arena: handle,
        slot: 0,
        generation: 0,
    };
    let arena = state.slabs.get_mut(&handle).expect("slab should exist");
    arena.slots.insert(0, RuntimeValue::Int(4));
    arena.generations.insert(0, 0);
    arena.next_slot = 1;
    arena.sealed = true;

    let mut scopes = vec![RuntimeScope::default()];
    insert_runtime_local(
        &mut state,
        0,
        &mut scopes[0],
        0,
        "borrow".to_string(),
        false,
        RuntimeValue::Ref(RuntimeReferenceValue {
            mode: RuntimeReferenceMode::Read,
            target: RuntimeReferenceTarget::SlabSlot {
                id,
                members: Vec::new(),
            },
        }),
    );

    let mut host = BufferedHost::default();
    let err = execute_runtime_intrinsic(
        RuntimeIntrinsic::MemorySlabUnseal,
        &[],
        &mut vec![RuntimeValue::Opaque(RuntimeOpaqueValue::Slab(handle))],
        &plan,
        Some(&mut scopes),
        None,
        None,
        None,
        None,
        &mut state,
        &mut host,
    )
    .expect_err("slab unseal should reject while borrows are live");
    assert!(err.contains("borrows"), "{err}");
}

#[test]
fn array_view_edit_rejects_conflicting_live_read_view() {
    let plan = empty_runtime_plan("demo");
    let mut state = RuntimeExecutionState::default();
    let mut scopes = vec![RuntimeScope::default()];
    insert_runtime_local(
        &mut state,
        0,
        &mut scopes[0],
        0,
        "values".to_string(),
        true,
        RuntimeValue::Array(vec![RuntimeValue::Int(1), RuntimeValue::Int(2)]),
    );
    let local = scopes[0]
        .locals
        .get("values")
        .expect("values local should exist")
        .handle;
    let reference = RuntimeReferenceValue {
        mode: RuntimeReferenceMode::Edit,
        target: RuntimeReferenceTarget::Local {
            local,
            members: Vec::new(),
        },
    };
    let read_view = insert_runtime_read_view_from_reference(
        &mut state,
        &["Int".to_string()],
        reference.clone(),
        0,
        1,
    );
    insert_runtime_local(
        &mut state,
        0,
        &mut scopes[0],
        1,
        "alias".to_string(),
        false,
        RuntimeValue::Opaque(RuntimeOpaqueValue::ReadView(read_view)),
    );

    let mut host = BufferedHost::default();
    let err = execute_runtime_intrinsic(
        RuntimeIntrinsic::MemoryArrayViewEdit,
        &["Int".to_string()],
        &mut vec![
            RuntimeValue::Ref(reference),
            RuntimeValue::Int(0),
            RuntimeValue::Int(1),
        ],
        &plan,
        Some(&mut scopes),
        Some("demo"),
        Some("demo"),
        None,
        None,
        &mut state,
        &mut host,
    )
    .expect_err("array_view_edit should reject conflicting live read views");
    assert!(err.contains("exclusive view acquisition"), "{err}");
}

#[test]
fn edit_view_subview_edit_allows_source_reborrow_without_other_aliases() {
    let plan = empty_runtime_plan("demo");
    let mut state = RuntimeExecutionState::default();
    let mut scopes = vec![RuntimeScope::default()];
    insert_runtime_local(
        &mut state,
        0,
        &mut scopes[0],
        0,
        "values".to_string(),
        true,
        RuntimeValue::Array(vec![RuntimeValue::Int(10), RuntimeValue::Int(20)]),
    );
    let local = scopes[0]
        .locals
        .get("values")
        .expect("values local should exist")
        .handle;
    let reference = RuntimeReferenceValue {
        mode: RuntimeReferenceMode::Edit,
        target: RuntimeReferenceTarget::Local {
            local,
            members: Vec::new(),
        },
    };

    let mut host = BufferedHost::default();
    let root_view = match execute_runtime_intrinsic(
        RuntimeIntrinsic::MemoryArrayViewEdit,
        &["Int".to_string()],
        &mut vec![
            RuntimeValue::Ref(reference),
            RuntimeValue::Int(0),
            RuntimeValue::Int(2),
        ],
        &plan,
        Some(&mut scopes),
        Some("demo"),
        Some("demo"),
        None,
        None,
        &mut state,
        &mut host,
    )
    .expect("root edit view should succeed")
    {
        RuntimeValue::Opaque(RuntimeOpaqueValue::EditView(handle)) => handle,
        other => panic!("expected edit view handle, got {other:?}"),
    };

    let nested = execute_runtime_intrinsic(
        RuntimeIntrinsic::MemoryEditViewSubviewEdit,
        &["Int".to_string()],
        &mut vec![
            RuntimeValue::Opaque(RuntimeOpaqueValue::EditView(root_view)),
            RuntimeValue::Int(0),
            RuntimeValue::Int(1),
        ],
        &plan,
        Some(&mut scopes),
        Some("demo"),
        Some("demo"),
        None,
        None,
        &mut state,
        &mut host,
    )
    .expect("subview edit should allow reborrow of the source handle");
    assert!(
        matches!(
            nested,
            RuntimeValue::Opaque(RuntimeOpaqueValue::EditView(_))
        ),
        "expected nested edit view, got {nested:?}"
    );
}

#[test]
fn frame_capacity_reset_on_frame_resets_without_recycle() {
    let handle = RuntimeFrameArenaHandle(1);
    let mut state = RuntimeExecutionState::default();
    state.frame_arenas.insert(
        handle,
        RuntimeFrameArenaState {
            type_args: vec!["Int".to_string()],
            next_slot: 2,
            generation: 7,
            slots: BTreeMap::from([(0, RuntimeValue::Int(1)), (1, RuntimeValue::Int(2))]),
            policy: RuntimeFrameArenaPolicy {
                base_capacity: 2,
                current_limit: 2,
                growth_step: 0,
                pressure: RuntimeMemoryPressurePolicy::Bounded,
                recycle: RuntimeFrameRecyclePolicy::Manual,
                reset_on: RuntimeResetOnPolicy::Frame,
            },
        },
    );

    let arena = state
        .frame_arenas
        .get_mut(&handle)
        .expect("frame arena should exist");
    ensure_runtime_frame_capacity(arena).expect("frame reset_on=frame should reset on saturation");
    assert_eq!(arena.generation, 8);
    assert_eq!(arena.next_slot, 0);
    assert!(arena.slots.is_empty());
    assert_eq!(arena.policy.current_limit, arena.policy.base_capacity);
}

#[test]
fn slab_capacity_growth_uses_page_size() {
    let mut arena = RuntimeSlabState {
        type_args: vec!["Int".to_string()],
        next_slot: 1,
        free_slots: Vec::new(),
        generations: BTreeMap::from([(0, 0)]),
        slots: BTreeMap::from([(0, RuntimeValue::Int(1))]),
        policy: RuntimeSlabPolicy {
            base_capacity: 1,
            current_limit: 1,
            growth_step: 0,
            pressure: RuntimeMemoryPressurePolicy::Elastic,
            handle: RuntimeMemoryHandlePolicy::Stable,
            page: 4,
        },
        sealed: false,
    };

    ensure_runtime_slab_capacity(&mut arena).expect("elastic slab should grow by page size");
    assert_eq!(arena.policy.current_limit, 5);
}

#[test]
fn owner_exit_resets_owner_bound_frame_specs() {
    let handle = RuntimeFrameArenaHandle(7);
    let mut state = RuntimeExecutionState::default();
    state.frame_arenas.insert(
        handle,
        RuntimeFrameArenaState {
            type_args: vec!["Int".to_string()],
            next_slot: 1,
            generation: 0,
            slots: BTreeMap::from([(0, RuntimeValue::Int(9))]),
            policy: RuntimeFrameArenaPolicy {
                base_capacity: 1,
                current_limit: 1,
                growth_step: 0,
                pressure: RuntimeMemoryPressurePolicy::Bounded,
                recycle: RuntimeFrameRecyclePolicy::Manual,
                reset_on: RuntimeResetOnPolicy::OwnerExit,
            },
        },
    );
    let mut scopes = vec![RuntimeScope {
        memory_specs: BTreeMap::from([(
            "scratch".to_string(),
            super::RuntimeMemorySpecState {
                spec: ExecMemorySpecDecl {
                    family: "frame".to_string(),
                    name: "scratch".to_string(),
                    default_modifier: None,
                    details: Vec::new(),
                },
                handle: Some(RuntimeValue::Opaque(RuntimeOpaqueValue::FrameArena(handle))),
                handle_policy: Some(RuntimeMemoryHandlePolicy::Stable),
                owner_keys: vec!["demo::Owner".to_string()],
            },
        )]),
        ..RuntimeScope::default()
    }];

    runtime_reset_owner_exit_memory_specs_in_scopes(&mut scopes, &mut state, "demo::Owner")
        .expect("owner_exit should reset owner-bound frame specs");
    let arena = state
        .frame_arenas
        .get(&handle)
        .expect("frame arena should still exist");
    assert!(arena.slots.is_empty());
    assert_eq!(arena.generation, 1);
}

#[test]
fn owner_exit_resets_owner_bound_temp_specs() {
    let handle = RuntimeTempArenaHandle(8);
    let mut state = RuntimeExecutionState::default();
    state.temp_arenas.insert(
        handle,
        super::RuntimeTempArenaState {
            type_args: vec!["Int".to_string()],
            next_slot: 1,
            generation: 0,
            slots: BTreeMap::from([(0, RuntimeValue::Int(5))]),
            policy: super::RuntimeTempArenaPolicy {
                base_capacity: 1,
                current_limit: 1,
                growth_step: 0,
                pressure: RuntimeMemoryPressurePolicy::Bounded,
                reset_on: RuntimeResetOnPolicy::OwnerExit,
            },
        },
    );
    let mut scopes = vec![RuntimeScope {
        memory_specs: BTreeMap::from([(
            "scratch".to_string(),
            super::RuntimeMemorySpecState {
                spec: ExecMemorySpecDecl {
                    family: "temp".to_string(),
                    name: "scratch".to_string(),
                    default_modifier: None,
                    details: Vec::new(),
                },
                handle: Some(RuntimeValue::Opaque(RuntimeOpaqueValue::TempArena(handle))),
                handle_policy: Some(RuntimeMemoryHandlePolicy::Stable),
                owner_keys: vec!["demo::Owner".to_string()],
            },
        )]),
        ..RuntimeScope::default()
    }];

    runtime_reset_owner_exit_memory_specs_in_scopes(&mut scopes, &mut state, "demo::Owner")
        .expect("owner_exit should reset owner-bound temp specs");
    let arena = state
        .temp_arenas
        .get(&handle)
        .expect("temp arena should still exist");
    assert!(arena.slots.is_empty());
    assert_eq!(arena.generation, 1);
}

#[test]
fn ring_window_rejects_requests_past_configured_window() {
    let plan = empty_runtime_plan("demo");
    let mut state = RuntimeExecutionState::default();
    let handle = insert_runtime_ring_buffer(
        &mut state,
        &["Int".to_string()],
        super::RuntimeRingBufferPolicy {
            window: 1,
            ..default_runtime_ring_policy(4)
        },
    );
    let arena = state
        .ring_buffers
        .get_mut(&handle)
        .expect("ring buffer should exist");
    arena.slots.insert(0, RuntimeValue::Int(1));
    arena.slots.insert(1, RuntimeValue::Int(2));
    arena.order.push_back(0);
    arena.order.push_back(1);
    arena.next_slot = 2;
    arena.generations.insert(0, 0);
    arena.generations.insert(1, 0);

    let mut host = BufferedHost::default();
    let err = execute_runtime_intrinsic(
        RuntimeIntrinsic::MemoryRingWindowRead,
        &["Int".to_string()],
        &mut vec![
            RuntimeValue::Opaque(RuntimeOpaqueValue::RingBuffer(handle)),
            RuntimeValue::Int(0),
            RuntimeValue::Int(2),
        ],
        &plan,
        None,
        None,
        None,
        None,
        None,
        &mut state,
        &mut host,
    )
    .expect_err("ring_window should reject requests beyond configured policy.window");
    assert!(err.contains("configured window `1`"), "{err}");
}

#[test]
fn ring_window_read_tracks_live_ring_updates() {
    let plan = empty_runtime_plan("demo");
    let mut state = RuntimeExecutionState::default();
    let handle = insert_runtime_ring_buffer(
        &mut state,
        &["Int".to_string()],
        default_runtime_ring_policy(4),
    );
    let arena = state
        .ring_buffers
        .get_mut(&handle)
        .expect("ring buffer should exist");
    arena.slots.insert(0, RuntimeValue::Int(21));
    arena.slots.insert(1, RuntimeValue::Int(22));
    arena.order.push_back(0);
    arena.order.push_back(1);
    arena.next_slot = 2;
    arena.generations.insert(0, 0);
    arena.generations.insert(1, 0);

    let mut host = BufferedHost::default();
    let read_view = match execute_runtime_intrinsic(
        RuntimeIntrinsic::MemoryRingWindowRead,
        &["Int".to_string()],
        &mut vec![
            RuntimeValue::Opaque(RuntimeOpaqueValue::RingBuffer(handle)),
            RuntimeValue::Int(0),
            RuntimeValue::Int(2),
        ],
        &plan,
        None,
        None,
        None,
        None,
        None,
        &mut state,
        &mut host,
    )
    .expect("ring_window_read should succeed")
    {
        RuntimeValue::Opaque(RuntimeOpaqueValue::ReadView(view)) => view,
        other => panic!("expected ReadView, got {other:?}"),
    };

    execute_runtime_intrinsic(
        RuntimeIntrinsic::MemoryRingSet,
        &[],
        &mut vec![
            RuntimeValue::Opaque(RuntimeOpaqueValue::RingBuffer(handle)),
            RuntimeValue::Opaque(RuntimeOpaqueValue::RingId(RuntimeRingIdValue {
                arena: handle,
                slot: 0,
                generation: 0,
            })),
            RuntimeValue::Int(55),
        ],
        &plan,
        None,
        None,
        None,
        None,
        None,
        &mut state,
        &mut host,
    )
    .expect("ring_set should succeed");

    let snapshot =
        runtime_read_view_snapshot(&state, read_view).expect("ring-backed view should stay live");
    assert_eq!(snapshot, vec![RuntimeValue::Int(55), RuntimeValue::Int(22)]);
}

#[test]
fn ring_window_edit_writes_through_to_ring_buffer() {
    let plan = empty_runtime_plan("demo");
    let mut state = RuntimeExecutionState::default();
    let handle = insert_runtime_ring_buffer(
        &mut state,
        &["Int".to_string()],
        default_runtime_ring_policy(4),
    );
    let arena = state
        .ring_buffers
        .get_mut(&handle)
        .expect("ring buffer should exist");
    arena.slots.insert(0, RuntimeValue::Int(21));
    arena.slots.insert(1, RuntimeValue::Int(22));
    arena.order.push_back(0);
    arena.order.push_back(1);
    arena.next_slot = 2;
    arena.generations.insert(0, 0);
    arena.generations.insert(1, 0);

    let mut host = BufferedHost::default();
    let edit_view = match execute_runtime_intrinsic(
        RuntimeIntrinsic::MemoryRingWindowEdit,
        &["Int".to_string()],
        &mut vec![
            RuntimeValue::Opaque(RuntimeOpaqueValue::RingBuffer(handle)),
            RuntimeValue::Int(0),
            RuntimeValue::Int(2),
        ],
        &plan,
        None,
        None,
        None,
        None,
        None,
        &mut state,
        &mut host,
    )
    .expect("ring_window_edit should succeed")
    {
        RuntimeValue::Opaque(RuntimeOpaqueValue::EditView(view)) => view,
        other => panic!("expected EditView, got {other:?}"),
    };

    execute_runtime_intrinsic(
        RuntimeIntrinsic::MemoryEditViewSet,
        &[],
        &mut vec![
            RuntimeValue::Opaque(RuntimeOpaqueValue::EditView(edit_view)),
            RuntimeValue::Int(1),
            RuntimeValue::Int(77),
        ],
        &plan,
        None,
        None,
        None,
        None,
        None,
        &mut state,
        &mut host,
    )
    .expect("edit_view_set should write through to the ring");

    let updated = state
        .ring_buffers
        .get(&handle)
        .and_then(|arena| arena.slots.get(&1))
        .cloned()
        .expect("ring slot should still exist");
    assert_eq!(updated, RuntimeValue::Int(77));
}

#[test]
fn ring_window_edit_rejects_overlapping_live_read_window() {
    let plan = empty_runtime_plan("demo");
    let mut state = RuntimeExecutionState::default();
    let handle = insert_runtime_ring_buffer(
        &mut state,
        &["Int".to_string()],
        default_runtime_ring_policy(4),
    );
    let arena = state
        .ring_buffers
        .get_mut(&handle)
        .expect("ring buffer should exist");
    arena.slots.insert(0, RuntimeValue::Int(1));
    arena.slots.insert(1, RuntimeValue::Int(2));
    arena.order.push_back(0);
    arena.order.push_back(1);
    arena.next_slot = 2;
    arena.generations.insert(0, 0);
    arena.generations.insert(1, 0);

    let mut scopes = vec![RuntimeScope::default()];
    let mut host = BufferedHost::default();
    let read_view = execute_runtime_intrinsic(
        RuntimeIntrinsic::MemoryRingWindowRead,
        &["Int".to_string()],
        &mut vec![
            RuntimeValue::Opaque(RuntimeOpaqueValue::RingBuffer(handle)),
            RuntimeValue::Int(0),
            RuntimeValue::Int(2),
        ],
        &plan,
        Some(&mut scopes),
        None,
        None,
        None,
        None,
        &mut state,
        &mut host,
    )
    .expect("ring_window_read should succeed");
    insert_runtime_local(
        &mut state,
        0,
        &mut scopes[0],
        0,
        "live_window".to_string(),
        false,
        read_view,
    );

    let err = execute_runtime_intrinsic(
        RuntimeIntrinsic::MemoryRingWindowEdit,
        &["Int".to_string()],
        &mut vec![
            RuntimeValue::Opaque(RuntimeOpaqueValue::RingBuffer(handle)),
            RuntimeValue::Int(0),
            RuntimeValue::Int(2),
        ],
        &plan,
        Some(&mut scopes),
        None,
        None,
        None,
        None,
        &mut state,
        &mut host,
    )
    .expect_err("ring_window_edit should reject overlapping live read windows");
    assert!(err.contains("exclusive view acquisition"), "{err}");
}

#[test]
fn ring_push_rejects_overwrite_while_live_ring_window_read_is_live() {
    let plan = empty_runtime_plan("demo");
    let mut state = RuntimeExecutionState::default();
    let handle = insert_runtime_ring_buffer(
        &mut state,
        &["Int".to_string()],
        default_runtime_ring_policy(2),
    );
    let arena = state
        .ring_buffers
        .get_mut(&handle)
        .expect("ring buffer should exist");
    arena.slots.insert(0, RuntimeValue::Int(1));
    arena.slots.insert(1, RuntimeValue::Int(2));
    arena.order.push_back(0);
    arena.order.push_back(1);
    arena.next_slot = 2;
    arena.generations.insert(0, 0);
    arena.generations.insert(1, 0);

    let mut scopes = vec![RuntimeScope::default()];
    let mut host = BufferedHost::default();
    let read_view = execute_runtime_intrinsic(
        RuntimeIntrinsic::MemoryRingWindowRead,
        &["Int".to_string()],
        &mut vec![
            RuntimeValue::Opaque(RuntimeOpaqueValue::RingBuffer(handle)),
            RuntimeValue::Int(0),
            RuntimeValue::Int(1),
        ],
        &plan,
        Some(&mut scopes),
        None,
        None,
        None,
        None,
        &mut state,
        &mut host,
    )
    .expect("ring_window_read should succeed");
    insert_runtime_local(
        &mut state,
        0,
        &mut scopes[0],
        0,
        "live_window".to_string(),
        false,
        read_view,
    );

    let err = execute_runtime_intrinsic(
        RuntimeIntrinsic::MemoryRingPush,
        &[],
        &mut vec![
            RuntimeValue::Opaque(RuntimeOpaqueValue::RingBuffer(handle)),
            RuntimeValue::Int(3),
        ],
        &plan,
        Some(&mut scopes),
        None,
        None,
        None,
        None,
        &mut state,
        &mut host,
    )
    .expect_err(
        "ring_push should reject overwrite while a ring window still covers the oldest slot",
    );
    assert!(err.contains("RingId"), "{err}");
}

#[test]
fn ring_window_read_rejects_split_capture() {
    let mut state = RuntimeExecutionState::default();
    let handle = insert_runtime_ring_buffer(
        &mut state,
        &["Int".to_string()],
        default_runtime_ring_policy(2),
    );
    let read_view = insert_runtime_read_view_from_ring_window(
        &mut state,
        &["Int".to_string()],
        handle,
        vec![0, 1],
        0,
        2,
    );
    let err = runtime_validate_split_value(
        &RuntimeValue::Opaque(RuntimeOpaqueValue::ReadView(read_view)),
        &state,
        "split",
    )
    .expect_err("split should reject RingBuffer-backed ReadView values");
    assert!(err.contains("move-only"), "{err}");
}

#[test]
fn session_reset_rejects_live_reference_views() {
    let plan = empty_runtime_plan("demo");
    let mut state = RuntimeExecutionState::default();
    let handle = insert_runtime_session_arena(
        &mut state,
        &["Int".to_string()],
        default_runtime_session_policy(4),
    );
    let id = RuntimeSessionIdValue {
        arena: handle,
        slot: 0,
        generation: 0,
    };
    let arena = state
        .session_arenas
        .get_mut(&handle)
        .expect("session arena should exist");
    arena
        .slots
        .insert(0, RuntimeValue::Array(vec![RuntimeValue::Int(1)]));
    arena.next_slot = 1;
    let _ = insert_runtime_read_view_from_reference(
        &mut state,
        &["Int".to_string()],
        RuntimeReferenceValue {
            mode: RuntimeReferenceMode::Read,
            target: RuntimeReferenceTarget::SessionSlot {
                id,
                members: Vec::new(),
            },
        },
        0,
        1,
    );

    let mut host = BufferedHost::default();
    let err = execute_runtime_intrinsic(
        RuntimeIntrinsic::MemorySessionReset,
        &[],
        &mut vec![RuntimeValue::Opaque(RuntimeOpaqueValue::SessionArena(
            handle,
        ))],
        &plan,
        None,
        None,
        None,
        None,
        None,
        &mut state,
        &mut host,
    )
    .expect_err("session reset should reject while borrowed views are live");
    assert!(err.contains("borrowed views"), "{err}");
}

#[test]
fn slab_remove_rejects_live_reference_views() {
    let plan = empty_runtime_plan("demo");
    let mut state = RuntimeExecutionState::default();
    let handle = insert_runtime_slab(
        &mut state,
        &["Int".to_string()],
        default_runtime_slab_policy(4),
    );
    let id = RuntimeSlabIdValue {
        arena: handle,
        slot: 0,
        generation: 0,
    };
    let arena = state.slabs.get_mut(&handle).expect("slab should exist");
    arena
        .slots
        .insert(0, RuntimeValue::Array(vec![RuntimeValue::Int(1)]));
    arena.generations.insert(0, 0);
    arena.next_slot = 1;
    let _ = insert_runtime_read_view_from_reference(
        &mut state,
        &["Int".to_string()],
        RuntimeReferenceValue {
            mode: RuntimeReferenceMode::Read,
            target: RuntimeReferenceTarget::SlabSlot {
                id,
                members: Vec::new(),
            },
        },
        0,
        1,
    );

    let mut host = BufferedHost::default();
    let err = execute_runtime_intrinsic(
        RuntimeIntrinsic::MemorySlabRemove,
        &[],
        &mut vec![
            RuntimeValue::Opaque(RuntimeOpaqueValue::Slab(handle)),
            RuntimeValue::Opaque(RuntimeOpaqueValue::SlabId(id)),
        ],
        &plan,
        None,
        None,
        None,
        None,
        None,
        &mut state,
        &mut host,
    )
    .expect_err("slab remove should reject while borrowed views are live");
    assert!(err.contains("borrowed views"), "{err}");
}

#[test]
fn ring_push_rejects_overwrite_while_live_reference_views() {
    let plan = empty_runtime_plan("demo");
    let mut state = RuntimeExecutionState::default();
    let handle = insert_runtime_ring_buffer(
        &mut state,
        &["Int".to_string()],
        default_runtime_ring_policy(1),
    );
    let id = RuntimeRingIdValue {
        arena: handle,
        slot: 0,
        generation: 0,
    };
    let arena = state
        .ring_buffers
        .get_mut(&handle)
        .expect("ring buffer should exist");
    arena
        .slots
        .insert(0, RuntimeValue::Array(vec![RuntimeValue::Int(1)]));
    arena.generations.insert(0, 0);
    arena.order.push_back(0);
    arena.next_slot = 1;
    let _ = insert_runtime_read_view_from_reference(
        &mut state,
        &["Int".to_string()],
        RuntimeReferenceValue {
            mode: RuntimeReferenceMode::Read,
            target: RuntimeReferenceTarget::RingSlot {
                id,
                members: Vec::new(),
            },
        },
        0,
        1,
    );

    let mut host = BufferedHost::default();
    let err = execute_runtime_intrinsic(
        RuntimeIntrinsic::MemoryRingPush,
        &[],
        &mut vec![
            RuntimeValue::Opaque(RuntimeOpaqueValue::RingBuffer(handle)),
            RuntimeValue::Int(2),
        ],
        &plan,
        None,
        None,
        None,
        None,
        None,
        &mut state,
        &mut host,
    )
    .expect_err("ring push should reject overwrite while borrowed views are live");
    assert!(err.contains("borrowed views"), "{err}");
}

#[test]
fn pool_compact_rejects_live_reference_views() {
    let plan = empty_runtime_plan("demo");
    let mut state = RuntimeExecutionState::default();
    let handle = insert_runtime_pool_arena(
        &mut state,
        &["Int".to_string()],
        default_runtime_pool_policy(4),
    );
    let id = RuntimePoolIdValue {
        arena: handle,
        slot: 0,
        generation: 0,
    };
    let arena = state
        .pool_arenas
        .get_mut(&handle)
        .expect("pool arena should exist");
    arena
        .slots
        .insert(0, RuntimeValue::Array(vec![RuntimeValue::Int(1)]));
    arena.generations.insert(0, 0);
    arena.next_slot = 1;
    let _ = insert_runtime_read_view_from_reference(
        &mut state,
        &["Int".to_string()],
        RuntimeReferenceValue {
            mode: RuntimeReferenceMode::Read,
            target: RuntimeReferenceTarget::PoolSlot {
                id,
                members: Vec::new(),
            },
        },
        0,
        1,
    );

    let mut host = BufferedHost::default();
    let err = execute_runtime_intrinsic(
        RuntimeIntrinsic::MemoryPoolCompact,
        &[],
        &mut vec![RuntimeValue::Opaque(RuntimeOpaqueValue::PoolArena(handle))],
        &plan,
        None,
        None,
        None,
        None,
        None,
        &mut state,
        &mut host,
    )
    .expect_err("pool compact should reject while borrowed views are live");
    assert!(err.contains("borrowed views"), "{err}");
}

#[test]
fn pool_compact_invalidates_all_old_ids_even_when_slot_stays_put() {
    let plan = empty_runtime_plan("demo");
    let mut state = RuntimeExecutionState::default();
    let handle = insert_runtime_pool_arena(
        &mut state,
        &["Int".to_string()],
        default_runtime_pool_policy(4),
    );
    let arena = state
        .pool_arenas
        .get_mut(&handle)
        .expect("pool arena should exist");
    arena.slots.insert(0, RuntimeValue::Int(10));
    arena.slots.insert(1, RuntimeValue::Int(20));
    arena.generations.insert(0, 0);
    arena.generations.insert(1, 0);
    arena.next_slot = 2;

    let old_a = RuntimePoolIdValue {
        arena: handle,
        slot: 0,
        generation: 0,
    };
    let old_b = RuntimePoolIdValue {
        arena: handle,
        slot: 1,
        generation: 0,
    };

    let mut host = BufferedHost::default();
    let relocations = execute_runtime_intrinsic(
        RuntimeIntrinsic::MemoryPoolCompact,
        &["Int".to_string()],
        &mut vec![RuntimeValue::Opaque(RuntimeOpaqueValue::PoolArena(handle))],
        &plan,
        None,
        None,
        None,
        None,
        None,
        &mut state,
        &mut host,
    )
    .expect("pool compact should succeed");

    let RuntimeValue::List(relocations) = relocations else {
        panic!("expected pool compact relocations list");
    };
    assert_eq!(
        relocations.len(),
        2,
        "every live entry should produce a relocation row"
    );

    let arena = state
        .pool_arenas
        .get(&handle)
        .expect("pool arena should exist");
    assert!(!pool_id_is_live(handle, arena, old_a));
    assert!(!pool_id_is_live(handle, arena, old_b));

    let new_ids = relocations
        .iter()
        .map(|row| match row {
            RuntimeValue::Record { fields, .. } => {
                let RuntimeValue::Opaque(RuntimeOpaqueValue::PoolId(old)) =
                    fields.get("old").expect("old field should exist")
                else {
                    panic!("expected old pool id");
                };
                let RuntimeValue::Opaque(RuntimeOpaqueValue::PoolId(new)) =
                    fields.get("new").expect("new field should exist")
                else {
                    panic!("expected new pool id");
                };
                (*old, *new)
            }
            other => panic!("expected relocation row, got {other:?}"),
        })
        .collect::<Vec<_>>();
    assert_eq!(new_ids[0].0, old_a);
    assert_eq!(new_ids[1].0, old_b);
    assert_eq!(new_ids[0].1.generation, 1);
    assert_eq!(new_ids[1].1.generation, 1);
    assert!(pool_id_is_live(handle, arena, new_ids[0].1));
    assert!(pool_id_is_live(handle, arena, new_ids[1].1));
}

#[test]
fn execute_main_runs_headed_regions_positive_workspace_fixture() {
    let fixture_root = repo_root()
        .join("conformance")
        .join("fixtures")
        .join("headed_regions_v1_workspace");
    let dir = temp_workspace_dir("headed_regions_positive_fixture");
    for relative_path in [
        "book.toml",
        "app/book.toml",
        "app/src/shelf.arc",
        "app/src/types.arc",
        "core/book.toml",
        "core/src/book.arc",
        "core/src/types.arc",
    ] {
        let source = fs::read_to_string(fixture_root.join(relative_path))
            .expect("headed regions positive fixture file should be readable");
        write_file(&dir.join(relative_path), &source);
    }

    let plan = build_workspace_plan_for_member(&dir, "app");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 17);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_consumes_named_recycle_owner_exits() {
    let dir = temp_workspace_dir("headed_region_owner_exit");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_headed_region_owner_exit\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "obj Counter:\n",
            "    value: Int\n",
            "create Session [Counter] scope-exit:\n",
            "    done: when false retain [Counter]\n",
            "fn main() -> Int:\n",
            "    if true:\n",
            "        let active = Session :: :: call\n",
            "        recycle -done\n",
            "            false\n",
            "        return 1\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_headed_region_owner_exit")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 7);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_bind_recovery_regions() {
    let dir = temp_workspace_dir("headed_region_bind_recovery");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_headed_region_bind_recovery\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.option\n",
            "import std.result\n",
            "use std.option.Option\n",
            "use std.result.Result\n",
            "fn main() -> Int:\n",
            "    let mut current = 5\n",
            "    bind -return 99\n",
            "        let fallback = Option.None[Int] :: :: call -default 7\n",
            "        current = Option.None[Int] :: :: call -preserve\n",
            "        current = Option.None[Int] :: :: call -replace 11\n",
            "        let ok = Result.Ok[Int, Str] :: 3 :: call\n",
            "    return fallback + current + ok\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_headed_region_bind_recovery")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 21);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_bind_require_loop_exits() {
    let dir = temp_workspace_dir("headed_region_bind_require_loop_exits");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_headed_region_bind_require_loop_exits\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "fn main() -> Int:\n",
            "    let mut i = 0\n",
            "    let mut sum = 0\n",
            "    while i < 5:\n",
            "        i = i + 1\n",
            "        bind -continue\n",
            "            require i != 3\n",
            "        sum = sum + i\n",
            "        bind -break\n",
            "            require sum <= 6\n",
            "    return sum\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_headed_region_bind_require_loop_exits")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 7);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_construct_regions_preserve_direct_values_and_payload_acquisition() {
    let dir = temp_workspace_dir("headed_region_construct_values");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_headed_region_construct_values\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.option\n",
            "import std.result\n",
            "use std.option.Option\n",
            "use std.result.Result\n",
            "record Widget:\n",
            "    ready: Bool\n",
            "    maybe: Option[Int]\n",
            "    outcome: Result[Int, Str]\n",
            "enum Packet:\n",
            "    Data(Int)\n",
            "fn main() -> Int:\n",
            "    let built = construct yield Widget -return 99\n",
            "        ready = false\n",
            "        maybe = Option.None[Int] :: :: call\n",
            "        outcome = Result.Err[Int, Str] :: \"bad\" :: call\n",
            "    let packet = construct yield Packet.Data -return 98\n",
            "        payload = Result.Ok[Int, Str] :: 7 :: call\n",
            "    if built.ready:\n",
            "        return 1\n",
            "    let maybe_ok = match built.maybe:\n",
            "        Option.Some(_) => false\n",
            "        Option.None => true\n",
            "    let outcome_ok = match built.outcome:\n",
            "        Result.Ok(_) => false\n",
            "        Result.Err(message) => message == \"bad\"\n",
            "    let packet_value = match packet:\n",
            "        Packet.Data(value) => value\n",
            "    if not maybe_ok:\n",
            "        return 2\n",
            "    if not outcome_ok:\n",
            "        return 3\n",
            "    return packet_value\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_headed_region_construct_values")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 7);

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
            "import arcana_process.io\n",
            "fn main() -> Int:\n",
            "    let local_x = 1\n",
            "    let mut local_y = 2\n",
            "    let x_ref = &read local_x\n",
            "    let y_ref = &edit local_y\n",
            "    let sum = *x_ref + *y_ref\n",
            "    arcana_process.io.print[Int] :: sum :: call\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_local_borrow")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(host.stdout, vec!["3".to_string()]);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_hold_capability_reclaim_flow() {
    let dir = temp_workspace_dir("hold_capability_reclaim");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_hold_capability_reclaim\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import arcana_process.io\n",
            "fn main() -> Int:\n",
            "    let mut x = 1\n",
            "    let held = &hold x\n",
            "    let snapshot = *held\n",
            "    arcana_process.io.print[Int] :: snapshot :: call\n",
            "    reclaim held\n",
            "    x = 2\n",
            "    arcana_process.io.print[Int] :: x :: call\n",
            "    return x\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_hold_capability_reclaim")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 2);
    assert_eq!(host.stdout, vec!["1".to_string(), "2".to_string()]);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_deferred_hold_reclaim_flow() {
    let dir = temp_workspace_dir("deferred_hold_reclaim");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_deferred_hold_reclaim\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import arcana_process.io\n",
            "fn main() -> Int:\n",
            "    let mut x = 1\n",
            "    if true:\n",
            "        let held = &hold x\n",
            "        defer reclaim held\n",
            "        let snapshot = *held\n",
            "        arcana_process.io.print[Int] :: snapshot :: call\n",
            "    x = 3\n",
            "    arcana_process.io.print[Int] :: x :: call\n",
            "    return x\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_deferred_hold_reclaim")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 3);
    assert_eq!(host.stdout, vec!["1".to_string(), "3".to_string()]);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_take_capability_once() {
    let dir = temp_workspace_dir("take_capability_once");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_take_capability_once\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import arcana_process.io\n",
            "fn main() -> Int:\n",
            "    let text = \"hi\"\n",
            "    let token = &take text\n",
            "    let value = *token\n",
            "    arcana_process.io.print[Str] :: value :: call\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_take_capability_once")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(host.stdout, vec!["hi".to_string()]);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_plain_hold_param_for_call_duration_only() {
    let dir = temp_workspace_dir("plain_hold_param");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_plain_hold_param\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import arcana_process.io\n",
            "fn inspect(hold value: Int) -> Int:\n",
            "    arcana_process.io.print[Int] :: value :: call\n",
            "    return value + 1\n",
            "fn main() -> Int:\n",
            "    let mut x = 4\n",
            "    let seen = inspect :: x :: call\n",
            "    x = 9\n",
            "    arcana_process.io.print[Int] :: seen :: call\n",
            "    arcana_process.io.print[Int] :: x :: call\n",
            "    return x\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_plain_hold_param")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 9);
    assert_eq!(
        host.stdout,
        vec!["4".to_string(), "5".to_string(), "9".to_string()]
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_assignment_through_edit_capability() {
    let dir = temp_workspace_dir("capability_deref_assign");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_capability_deref_assign\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import arcana_process.io\n",
            "fn main() -> Int:\n",
            "    let mut x = 1\n",
            "    let edit_cap = &edit x\n",
            "    *edit_cap = 3\n",
            "    arcana_process.io.print[Int] :: *edit_cap :: call\n",
            "    return *edit_cap\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_capability_deref_assign")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 3);
    assert_eq!(host.stdout, vec!["3".to_string()]);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn runtime_hold_capability_member_assignment_updates_target() {
    let plan = empty_runtime_plan("demo");
    let mut state = RuntimeExecutionState::default();
    let mut scopes = vec![RuntimeScope::default()];
    insert_runtime_local(
        &mut state,
        0,
        &mut scopes[0],
        0,
        "counter".to_string(),
        true,
        RuntimeValue::Record {
            name: "Counter".to_string(),
            fields: BTreeMap::from([("value".to_string(), RuntimeValue::Int(1))]),
        },
    );
    let counter_handle = scopes[0]
        .locals
        .get("counter")
        .expect("counter local should exist")
        .handle;
    scopes[0]
        .locals
        .get_mut("counter")
        .expect("counter local should exist")
        .held = true;
    insert_runtime_local(
        &mut state,
        0,
        &mut scopes[0],
        1,
        "held".to_string(),
        false,
        RuntimeValue::Ref(RuntimeReferenceValue {
            mode: RuntimeReferenceMode::Hold,
            target: RuntimeReferenceTarget::Local {
                local: counter_handle,
                members: Vec::new(),
            },
        }),
    );

    let mut host = BufferedHost::default();
    write_assign_target_value_runtime(
        &ParsedAssignTarget::Member {
            target: Box::new(ParsedAssignTarget::Name("held".to_string())),
            member: "value".to_string(),
        },
        RuntimeValue::Int(4),
        &plan,
        "demo",
        "demo",
        &mut scopes,
        &BTreeMap::new(),
        &RuntimeTypeBindings::default(),
        &mut state,
        &mut host,
    )
    .expect("assignment through hold capability member should succeed");

    let value = read_runtime_reference(
        &mut scopes,
        &plan,
        "demo",
        "demo",
        &BTreeMap::new(),
        &RuntimeTypeBindings::default(),
        &mut state,
        &RuntimeReferenceValue {
            mode: RuntimeReferenceMode::Hold,
            target: RuntimeReferenceTarget::Local {
                local: counter_handle,
                members: vec!["value".to_string()],
            },
        },
        &mut host,
    )
    .expect("hold member reference should read through hold capability");
    assert_eq!(value, RuntimeValue::Int(4));

    reclaim_held_target_local(
        &mut scopes,
        &RuntimeReferenceTarget::Local {
            local: counter_handle,
            members: Vec::new(),
        },
    )
    .expect("hold target should reclaim");
    reclaim_hold_capability_root_local(&mut scopes, &ParsedExpr::Path(vec!["held".to_string()]))
        .expect("hold token should reclaim");
    validate_scope_hold_tokens(&scopes[0]).expect("hold token should no longer be live");
}

#[test]
fn runtime_reclaim_helpers_release_hold_target_and_token() {
    let x_handle = RuntimeLocalHandle(0);
    let held_handle = RuntimeLocalHandle(1);
    let hold_target = RuntimeReferenceTarget::Local {
        local: x_handle,
        members: Vec::new(),
    };
    let mut scopes = vec![RuntimeScope {
        locals: BTreeMap::from([
            (
                "x".to_string(),
                RuntimeLocal {
                    binding_id: 0,
                    handle: x_handle,
                    mutable: true,
                    moved: false,
                    held: true,
                    take_reserved: false,
                    value: RuntimeValue::Int(1),
                },
            ),
            (
                "held".to_string(),
                RuntimeLocal {
                    binding_id: 1,
                    handle: held_handle,
                    mutable: false,
                    moved: false,
                    held: false,
                    take_reserved: false,
                    value: RuntimeValue::Ref(RuntimeReferenceValue {
                        mode: RuntimeReferenceMode::Hold,
                        target: hold_target.clone(),
                    }),
                },
            ),
        ]),
        ..RuntimeScope::default()
    }];

    reclaim_held_target_local(&mut scopes, &hold_target).expect("hold target should reclaim");
    reclaim_hold_capability_root_local(&mut scopes, &ParsedExpr::Path(vec!["held".to_string()]))
        .expect("hold token should reclaim");

    let x = scopes[0].locals.get("x").expect("x should remain");
    assert!(!x.held, "hold target should no longer be suspended");
    let held = scopes[0]
        .locals
        .get("held")
        .expect("held token should remain");
    assert!(held.moved, "hold token should be consumed");
    assert_eq!(held.value, RuntimeValue::Unit);
}

#[test]
fn runtime_redeem_take_reference_marks_target_moved() {
    let text_handle = RuntimeLocalHandle(0);
    let mut scopes = vec![RuntimeScope {
        locals: BTreeMap::from([(
            "text".to_string(),
            RuntimeLocal {
                binding_id: 0,
                handle: text_handle,
                mutable: false,
                moved: false,
                held: false,
                take_reserved: true,
                value: RuntimeValue::Str("hi".to_string()),
            },
        )]),
        ..RuntimeScope::default()
    }];

    redeem_take_reference(
        &mut scopes,
        &RuntimeReferenceValue {
            mode: RuntimeReferenceMode::Take,
            target: RuntimeReferenceTarget::Local {
                local: text_handle,
                members: Vec::new(),
            },
        },
    )
    .expect("take capability should redeem");

    let text = scopes[0].locals.get("text").expect("text should remain");
    assert!(text.moved, "take target should be marked moved");
    assert!(
        !text.take_reserved,
        "take reservation should clear after redeem"
    );
}

#[test]
fn validate_scope_hold_tokens_rejects_unreclaimed_hold_capability() {
    let mut scope = RuntimeScope::default();
    scope.locals.insert(
        "held".to_string(),
        RuntimeLocal {
            binding_id: 0,
            handle: RuntimeLocalHandle(1),
            mutable: false,
            moved: false,
            held: false,
            take_reserved: false,
            value: RuntimeValue::Ref(RuntimeReferenceValue {
                mode: RuntimeReferenceMode::Hold,
                target: RuntimeReferenceTarget::Local {
                    local: RuntimeLocalHandle(2),
                    members: Vec::new(),
                },
            }),
        },
    );

    let err = validate_scope_hold_tokens(&scope)
        .expect_err("scope validation should reject unreclaimed hold tokens");
    assert!(
        err.contains("local `held` holds an unreclaimed `&hold` capability at scope exit"),
        "{err}"
    );
}

#[test]
fn execute_main_runs_record_headed_regions_with_base_copy() {
    let dir = temp_workspace_dir("record_headed_regions");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_record_headed_regions\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "use std.option.Option\n",
            "use std.result.Result\n",
            "record Widget:\n",
            "    ready: Bool\n",
            "    maybe: Option[Int]\n",
            "    outcome: Result[Int, Str]\n",
            "fn main() -> Int:\n",
            "    let base = construct yield Widget -return 99\n",
            "        ready = false\n",
            "        maybe = Option.None[Int] :: :: call\n",
            "        outcome = Result.Err[Int, Str] :: \"bad\" :: call\n",
            "    let built = record yield Widget from base -return 99\n",
            "        ready = true\n",
            "    record deliver Widget from built -> mirrored -return 98\n",
            "        maybe = Option.Some[Int] :: 7 :: call\n",
            "    let mut placed = construct yield Widget -return 97\n",
            "        ready = false\n",
            "        maybe = Option.None[Int] :: :: call\n",
            "        outcome = Result.Ok[Int, Str] :: 0 :: call\n",
            "    record place Widget from mirrored -> placed -return 97\n",
            "        ready = mirrored.ready\n",
            "    if not placed.ready:\n",
            "        return 1\n",
            "    let maybe_value = match placed.maybe:\n",
            "        Option.Some(value) => value\n",
            "        Option.None => 0\n",
            "    let outcome_ok = match placed.outcome:\n",
            "        Result.Ok(_) => false\n",
            "        Result.Err(message) => message == \"bad\"\n",
            "    if not outcome_ok:\n",
            "        return 2\n",
            "    return maybe_value\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_record_headed_regions")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 7);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_record_headed_regions_with_cross_record_lift() {
    let dir = temp_workspace_dir("runtime_record_headed_regions_cross_record");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_record_headed_regions_cross_record\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "record Seed:\n",
            "    title: Str\n",
            "    count: Int\n",
            "record Widget:\n",
            "    title: Str\n",
            "    count: Int\n",
            "    ready: Bool\n",
            "fn main() -> Int:\n",
            "    let base = Seed :: title = \"seed\", count = 4 :: call\n",
            "    let built = record yield Widget from base -return 77\n",
            "        ready = true\n",
            "    if built.title != \"seed\":\n",
            "        return 1\n",
            "    if built.count != 4:\n",
            "        return 2\n",
            "    if not built.ready:\n",
            "        return 3\n",
            "    return built.count\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_record_headed_regions_cross_record")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 4);

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
            "import arcana_process.io\n",
            "async fn worker(value: Int) -> Int:\n",
            "    return value + 1\n",
            "fn helper(value: Int) -> Int:\n",
            "    return value * 2\n",
            "fn main() -> Int:\n",
            "    let task = weave worker :: 41 :: call\n",
            "    let thread = split helper :: 7 :: call\n",
            "    arcana_process.io.print[Bool] :: (task :: :: done) :: call\n",
            "    arcana_process.io.print[Bool] :: (thread :: :: done) :: call\n",
            "    arcana_process.io.print[Int] :: (task :: :: join) :: call\n",
            "    arcana_process.io.print[Int] :: (thread :: :: join) :: call\n",
            "    let awaited_task = task >> await\n",
            "    let awaited_thread = thread >> await\n",
            "    arcana_process.io.print[Int] :: awaited_task :: call\n",
            "    arcana_process.io.print[Int] :: awaited_thread :: call\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_std_concurrent_async")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
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
            "import arcana_process.io\n",
            "async fn compute() -> Int:\n",
            "    return 5\n",
            "async fn main() -> Int:\n",
            "    arcana_process.io.print[Int] :: (compute :: :: call) :: call\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_async_main")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
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
            "import arcana_process.io\n",
            "fn main() -> Int:\n",
            "    let task = weave 7\n",
            "    let thread = split 8\n",
            "    arcana_process.io.print[Bool] :: (task :: :: done) :: call\n",
            "    arcana_process.io.print[Bool] :: (thread :: :: done) :: call\n",
            "    arcana_process.io.print[Int] :: (task :: :: join) :: call\n",
            "    arcana_process.io.print[Int] :: (thread :: :: join) :: call\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_spawned_values_pending")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
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
            "import arcana_process.io\n",
            "fn worker() -> Int:\n",
            "    return std.concurrent.thread_id :: :: call\n",
            "fn main() -> Int:\n",
            "    arcana_process.io.print[Int] :: (std.concurrent.thread_id :: :: call) :: call\n",
            "    let thread = split worker :: :: call\n",
            "    arcana_process.io.print[Bool] :: (thread :: :: done) :: call\n",
            "    arcana_process.io.print[Int] :: (thread :: :: join) :: call\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_split_thread_id")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
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
fn execute_main_split_rejects_unsealed_session_capture() {
    let dir = temp_workspace_dir("split_unsealed_session_capture");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_split_unsealed_session_capture\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.memory\n",
            "fn worker(read arena: std.memory.SessionArena[Int]) -> Int:\n",
            "    return arena :: :: len\n",
            "fn main() -> Int:\n",
            "    let arena = std.memory.session_new[Int] :: 2 :: call\n",
            "    let thread = split worker :: arena :: call\n",
            "    return thread :: :: join\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);
    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_split_unsealed_session_capture")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let err =
        execute_main(&plan, &mut host).expect_err("split should reject unsealed session capture");
    assert!(err.contains("while sealed"), "{err}");

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_split_allows_sealed_session_capture() {
    let dir = temp_workspace_dir("split_sealed_session_capture");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_split_sealed_session_capture\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.memory\n",
            "fn worker(read arena: std.memory.SessionArena[Int]) -> Bool:\n",
            "    return arena :: :: is_sealed\n",
            "fn main() -> Int:\n",
            "    let mut arena = std.memory.session_new[Int] :: 2 :: call\n",
            "    arena :: :: seal\n",
            "    let thread = split worker :: arena :: call\n",
            "    if not (thread :: :: join):\n",
            "        return 1\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);
    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_split_sealed_session_capture")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("sealed session capture should execute");
    assert_eq!(code, 0);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_split_rejects_unsealed_slab_capture() {
    let dir = temp_workspace_dir("split_unsealed_slab_capture");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_split_unsealed_slab_capture\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.memory\n",
            "fn worker(read arena: std.memory.Slab[Int]) -> Int:\n",
            "    return arena :: :: len\n",
            "fn main() -> Int:\n",
            "    let arena = std.memory.slab_new[Int] :: 2 :: call\n",
            "    let thread = split worker :: arena :: call\n",
            "    return thread :: :: join\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);
    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_split_unsealed_slab_capture")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let err =
        execute_main(&plan, &mut host).expect_err("split should reject unsealed slab capture");
    assert!(err.contains("while sealed"), "{err}");

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_split_allows_sealed_slab_capture() {
    let dir = temp_workspace_dir("split_sealed_slab_capture");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_split_sealed_slab_capture\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.memory\n",
            "fn worker(read arena: std.memory.Slab[Int]) -> Bool:\n",
            "    return arena :: :: is_sealed\n",
            "fn main() -> Int:\n",
            "    let mut arena = std.memory.slab_new[Int] :: 2 :: call\n",
            "    arena :: :: seal\n",
            "    let thread = split worker :: arena :: call\n",
            "    if not (thread :: :: join):\n",
            "        return 1\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);
    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_split_sealed_slab_capture")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("sealed slab capture should execute");
    assert_eq!(code, 0);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_split_rejects_ring_capture() {
    let dir = temp_workspace_dir("split_ring_capture");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_split_ring_capture\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.memory\n",
            "fn worker(read arena: std.memory.RingBuffer[Int]) -> Int:\n",
            "    return arena :: :: len\n",
            "fn main() -> Int:\n",
            "    let arena = std.memory.ring_new[Int] :: 2 :: call\n",
            "    let thread = split worker :: arena :: call\n",
            "    return thread :: :: join\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);
    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_split_ring_capture")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let err = execute_main(&plan, &mut host)
        .expect_err("split should reject implicit ring capture without explicit move");
    assert!(err.contains("explicit move"), "{err}");

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_split_allows_take_ring_capture() {
    let dir = temp_workspace_dir("split_take_ring_capture");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_split_take_ring_capture\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.memory\n",
            "fn worker(take arena: std.memory.RingBuffer[Int]) -> Int:\n",
            "    return arena :: :: len\n",
            "fn main() -> Int:\n",
            "    let mut arena = std.memory.ring_new[Int] :: 2 :: call\n",
            "    arena :: 5 :: push\n",
            "    let thread = split worker :: arena :: call\n",
            "    return thread :: :: join\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);
    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_split_take_ring_capture")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("split should allow explicit take ring move");
    assert_eq!(code, 1);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn channel_send_rejects_unsealed_session_memory() {
    let plan = empty_runtime_plan("demo");
    let mut state = RuntimeExecutionState::default();
    let channel = insert_runtime_channel(&mut state, &["Int".to_string()], 2);
    let session = insert_runtime_session_arena(
        &mut state,
        &["Int".to_string()],
        default_runtime_session_policy(2),
    );
    let mut host = BufferedHost::default();
    let err = execute_runtime_intrinsic(
        RuntimeIntrinsic::ConcurrentChannelSend,
        &[],
        &mut vec![
            RuntimeValue::Opaque(RuntimeOpaqueValue::Channel(channel)),
            RuntimeValue::Opaque(RuntimeOpaqueValue::SessionArena(session)),
        ],
        &plan,
        None,
        None,
        None,
        None,
        None,
        &mut state,
        &mut host,
    )
    .expect_err("channel_send should reject unsealed session memory");
    assert!(err.contains("while sealed"), "{err}");
}

#[test]
fn mutex_put_rejects_ring_memory() {
    let plan = empty_runtime_plan("demo");
    let mut state = RuntimeExecutionState::default();
    let mutex_value = RuntimeValue::Int(0);
    let mut host = BufferedHost::default();
    let mutex = match execute_runtime_intrinsic(
        RuntimeIntrinsic::ConcurrentMutexNew,
        &["Int".to_string()],
        &mut vec![mutex_value],
        &plan,
        None,
        None,
        None,
        None,
        None,
        &mut state,
        &mut host,
    )
    .expect("mutex_new should succeed for ints")
    {
        RuntimeValue::Opaque(RuntimeOpaqueValue::Mutex(handle)) => handle,
        other => panic!("expected mutex handle, got {other:?}"),
    };
    let ring = insert_runtime_ring_buffer(
        &mut state,
        &["Int".to_string()],
        default_runtime_ring_policy(2),
    );
    let err = execute_runtime_intrinsic(
        RuntimeIntrinsic::ConcurrentMutexPut,
        &[],
        &mut vec![
            RuntimeValue::Opaque(RuntimeOpaqueValue::Mutex(mutex)),
            RuntimeValue::Opaque(RuntimeOpaqueValue::RingBuffer(ring)),
        ],
        &plan,
        None,
        None,
        None,
        None,
        None,
        &mut state,
        &mut host,
    )
    .expect_err("mutex_put should reject ring memory");
    assert!(err.contains("move-only"), "{err}");
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
            "import arcana_process.io\n",
            "fn seed() -> Int:\n",
            "    return 2\n",
            "fn inc(value: Int) -> Int:\n",
            "    return value + 1\n",
            "fn mul(value: Int) -> Int:\n",
            "    return value * 2\n",
            "fn main() -> Int:\n",
            "    let pipeline = forward :=> seed => inc => mul\n",
            "    arcana_process.io.print[Int] :: pipeline :: call\n",
            "    let fanout = parallel :=> seed => inc => mul\n",
            "    arcana_process.io.print[Int] :: fanout[0] :: call\n",
            "    arcana_process.io.print[Int] :: fanout[1] :: call\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_chain")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
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
fn execute_main_parallel_chain_stages_observe_overlap() {
    let dir = temp_workspace_dir("chain_runtime_parallel_overlap");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_chain_parallel_overlap\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.concurrent\n",
            "import arcana_process.io\n",
            "fn seed() -> Int:\n",
            "    return 1\n",
            "fn observe(value: Int, read started: AtomicInt, read overlap: AtomicBool) -> Int:\n",
            "    let already = started :: 1 :: add\n",
            "    if already > 0:\n",
            "        overlap :: true :: store\n",
            "    std.concurrent.sleep :: 250 :: call\n",
            "    started :: 1 :: sub\n",
            "    return value + 1\n",
            "fn main() -> Int:\n",
            "    let started = std.concurrent.atomic_int :: 0 :: call\n",
            "    let overlap = std.concurrent.atomic_bool :: false :: call\n",
            "    let fanout = parallel :=> seed => observe with (started, overlap) => observe with (started, overlap)\n",
            "    arcana_process.io.print[Int] :: fanout[0] :: call\n",
            "    arcana_process.io.print[Int] :: fanout[1] :: call\n",
            "    arcana_process.io.print[Bool] :: (overlap :: :: load) :: call\n",
            "    return 0\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let plan = build_workspace_plan_for_member(&dir, "runtime_chain_parallel_overlap");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("parallel chain should execute");

    assert_eq!(code, 0);
    assert_eq!(
        host.stdout,
        vec!["2".to_string(), "2".to_string(), "true".to_string()]
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_collect_broadcast_plan_and_async_chain_styles() {
    let dir = temp_workspace_dir("chain_runtime_styles");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_chain_styles\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import arcana_process.io\n",
            "fn seed() -> Int:\n",
            "    return 2\n",
            "fn inc(value: Int) -> Int:\n",
            "    return value + 1\n",
            "fn mul(value: Int) -> Int:\n",
            "    return value * 2\n",
            "async fn seed_async() -> Int:\n",
            "    return 2\n",
            "async fn inc_async(value: Int) -> Int:\n",
            "    return value + 1\n",
            "async fn mul_async(value: Int) -> Int:\n",
            "    return value * 2\n",
            "fn main() -> Int:\n",
            "    arcana_process.io.print[Int] :: (async :=> seed_async => inc_async => mul_async) :: call\n",
            "    let broadcasted = broadcast :=> seed => inc => mul\n",
            "    arcana_process.io.print[Int] :: broadcasted[0] :: call\n",
            "    arcana_process.io.print[Int] :: broadcasted[1] :: call\n",
            "    let collected = collect :=> seed => inc => mul\n",
            "    arcana_process.io.print[Int] :: collected[0] :: call\n",
            "    arcana_process.io.print[Int] :: collected[1] :: call\n",
            "    let planned = plan :=> seed => inc => mul\n",
            "    arcana_process.io.print[Int] :: planned :: call\n",
            "    return 0\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let plan = build_workspace_plan_for_member(&dir, "runtime_chain_styles");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(
        host.stdout,
        vec![
            "6".to_string(),
            "3".to_string(),
            "4".to_string(),
            "3".to_string(),
            "6".to_string(),
            "2".to_string(),
        ]
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_forces_lazy_chain_once_and_skips_unused_values() {
    let dir = temp_workspace_dir("chain_runtime_lazy");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_chain_lazy\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import arcana_process.io\n",
            "fn seed() -> Int:\n",
            "    arcana_process.io.print[Str] :: \"seed\" :: call\n",
            "    return 2\n",
            "fn inc(value: Int) -> Int:\n",
            "    return value + 1\n",
            "fn mul(value: Int) -> Int:\n",
            "    return value * 2\n",
            "fn main() -> Int:\n",
            "    let unused = lazy :=> seed => inc => mul\n",
            "    let forced = lazy :=> seed => inc => mul\n",
            "    arcana_process.io.print[Int] :: forced :: call\n",
            "    arcana_process.io.print[Int] :: forced :: call\n",
            "    return 0\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let plan = build_workspace_plan_for_member(&dir, "runtime_chain_lazy");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(
        host.stdout,
        vec!["seed".to_string(), "6".to_string(), "6".to_string()]
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_tuple_destructuring_in_let_and_for() {
    let dir = temp_workspace_dir("tuple_destructure_runtime");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_tuple_destructure\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "fn main() -> Int:\n",
            "    let pair = (2, 3)\n",
            "    let (left, right) = pair\n",
            "    for (first, second) in [(left, right)]:\n",
            "        return first + second\n",
            "    return 0\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let plan = build_workspace_plan_for_member(&dir, "runtime_tuple_destructure");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 5);

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
            "import std.text\n",
            "import arcana_process.env\n",
            "import arcana_process.io\n",
            "import std.text\n",
            "use std.result.Result\n",
            "fn main() -> Int:\n",
            "    let label = arcana_process.env.get_or :: \"ARCANA_LABEL\", \"unset\" :: call\n",
            "    let input = match (arcana_process.io.read_line :: :: call):\n",
            "        Result.Ok(value) => value\n",
            "        Result.Err(err) => err\n",
            "    let lines = std.text.split_lines :: \"alpha\\r\\nbeta\\n\" :: call\n",
            "    let bytes = std.text.bytes_from_str_utf8 :: input :: call\n",
            "    let mid = std.text.bytes_slice :: bytes, 1, 4 :: call\n",
            "    arcana_process.io.flush_stdout :: :: call\n",
            "    arcana_process.io.flush_stderr :: :: call\n",
            "    arcana_process.io.print[Str] :: label :: call\n",
            "    arcana_process.io.print[Bool] :: (std.text.starts_with :: input, \"he\" :: call) :: call\n",
            "    arcana_process.io.print[Bool] :: (std.text.ends_with :: input, \"lo\" :: call) :: call\n",
            "    arcana_process.io.print[Int] :: (lines :: :: len) :: call\n",
            "    arcana_process.io.print[Str] :: (std.text.from_int :: (std.text.bytes_len :: bytes :: call) :: call) :: call\n",
            "    arcana_process.io.print[Int] :: (std.text.bytes_at :: bytes, 1 :: call) :: call\n",
            "    arcana_process.io.print[Str] :: (std.text.bytes_to_str_utf8 :: mid :: call) :: call\n",
            "    arcana_process.io.print[Str] :: (std.text.bytes_sha256_hex :: bytes :: call) :: call\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_std_host_misc")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
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
            "import std.text\n",
            "import std.collections.array\n",
            "import std.collections.list\n",
            "import std.collections.map\n",
            "import std.collections.set\n",
            "import arcana_process.io\n",
            "import arcana_process.path\n",
            "import std.text\n",
            "import std.time\n",
            "import std.types.core\n",
            "use std.text as text\n",
            "use std.collections.array as arrays\n",
            "use std.collections.list as lists\n",
            "use std.collections.map as maps\n",
            "use std.collections.set as sets\n",
            "use arcana_process.path as paths\n",
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
            "    arcana_process.io.print[Bool] :: (paths.is_absolute :: norm :: call) :: call\n",
            "    arcana_process.io.print[Str] :: (paths.parent :: norm :: call) :: call\n",
            "    arcana_process.io.print[Str] :: (paths.file_name :: norm :: call) :: call\n",
            "    arcana_process.io.print[Str] :: (paths.ext :: norm :: call) :: call\n",
            "    arcana_process.io.print[Str] :: (unwrap_str :: (paths.stem :: norm :: call) :: call) :: call\n",
            "    arcana_process.io.print[Str] :: (paths.with_ext :: norm, \"bin\" :: call) :: call\n",
            "    arcana_process.io.print[Str] :: (unwrap_str :: (paths.relative_to :: norm, cwd :: call) :: call) :: call\n",
            "    arcana_process.io.print[Str] :: (unwrap_str :: (paths.strip_prefix :: norm, cwd :: call) :: call) :: call\n",
            "    arcana_process.io.print[Str] :: (paths.file_name :: (unwrap_str :: (paths.canonicalize :: file :: call) :: call) :: call) :: call\n",
            "    let trimmed = texts.trim :: \"  alpha,beta  \" :: call\n",
            "    let parts = texts.split :: trimmed, \",\" :: call\n",
            "    arcana_process.io.print[Int] :: (parts :: :: len) :: call\n",
            "    arcana_process.io.print[Str] :: (texts.join :: parts, \"+\" :: call) :: call\n",
            "    arcana_process.io.print[Str] :: (texts.repeat :: \"ha\", 3 :: call) :: call\n",
            "    arcana_process.io.print[Int] :: (unwrap_int :: (texts.to_int :: \"  -42 \" :: call) :: call) :: call\n",
            "    let arc = bytes.from_str_utf8 :: \"arcana\" :: call\n",
            "    let prefix = bytes.from_str_utf8 :: \"arc\" :: call\n",
            "    let can = bytes.from_str_utf8 :: \"can\" :: call\n",
            "    let na = bytes.from_str_utf8 :: \"na\" :: call\n",
            "    arcana_process.io.print[Bool] :: (bytes.starts_with :: arc, prefix :: call) :: call\n",
            "    arcana_process.io.print[Bool] :: (bytes.ends_with :: arc, na :: call) :: call\n",
            "    arcana_process.io.print[Int] :: (bytes.find :: arc, 0, can :: call) :: call\n",
            "    arcana_process.io.print[Bool] :: (bytes.contains :: arc, can :: call) :: call\n",
            "    let mut buf = bytes.new_buf :: :: call\n",
            "    arcana_process.io.print[Bool] :: ((bytes.buf_push :: buf, 65 :: call) :: :: is_ok) :: call\n",
            "    arcana_process.io.print[Int] :: (unwrap_int :: (bytes.buf_extend :: buf, can :: call) :: call) :: call\n",
            "    let combo = bytes.concat :: prefix, (bytes.buf_to_array :: buf :: call) :: call\n",
            "    arcana_process.io.print[Str] :: (bytes.to_str_utf8 :: combo :: call) :: call\n",
            "    let pos = core.vec2 :: 3, 4 :: call\n",
            "    let size = core.size2 :: 5, 6 :: call\n",
            "    let rect = core.rect :: pos, size :: call\n",
            "    let color = core.rgb :: 7, 8, 9 :: call\n",
            "    arcana_process.io.print[Int] :: (rect.pos.x + rect.size.h) :: call\n",
            "    arcana_process.io.print[Int] :: color.g :: call\n",
            "    let start = times.monotonic_now_ms :: :: call\n",
            "    let end = times.monotonic_now_ms :: :: call\n",
            "    let elapsed = times.elapsed_ms :: start, end :: call\n",
            "    times.sleep :: elapsed :: call\n",
            "    times.sleep_ms :: 3 :: call\n",
            "    arcana_process.io.print[Int] :: elapsed.value :: call\n",
            "    arcana_process.io.print[Int] :: (times.monotonic_now_ns :: :: call) :: call\n",
            "    let arr = arrays.new[Int] :: 3, 4 :: call\n",
            "    let arr_list = arr :: :: to_list\n",
            "    arcana_process.io.print[Int] :: (arr_list :: :: len) :: call\n",
            "    let mut xs = lists.new[Int] :: :: call\n",
            "    xs :: arr :: extend_array\n",
            "    let mut ys = lists.new[Int] :: :: call\n",
            "    ys :: 9 :: push\n",
            "    xs :: ys :: extend_list\n",
            "    arcana_process.io.print[Int] :: (xs :: :: len) :: call\n",
            "    ys :: :: clear\n",
            "    arcana_process.io.print[Bool] :: (ys :: :: is_empty) :: call\n",
            "    let pop_pair = xs :: 0 :: try_pop_or\n",
            "    arcana_process.io.print[Bool] :: pop_pair.0 :: call\n",
            "    arcana_process.io.print[Int] :: pop_pair.1 :: call\n",
            "    let mut mapping = maps.new[Str, Int] :: :: call\n",
            "    mapping :: \"a\", 1 :: set\n",
            "    mapping :: \"b\", 2 :: set\n",
            "    arcana_process.io.print[Int] :: ((maps.keys[Str, Int] :: mapping :: call) :: :: len) :: call\n",
            "    arcana_process.io.print[Int] :: ((maps.values[Str, Int] :: mapping :: call) :: :: len) :: call\n",
            "    arcana_process.io.print[Int] :: ((maps.items[Str, Int] :: mapping :: call) :: :: len) :: call\n",
            "    let mut set = sets.new[Int] :: :: call\n",
            "    set :: 5 :: insert\n",
            "    set :: 6 :: insert\n",
            "    arcana_process.io.print[Int] :: ((sets.items[Int] :: set :: call) :: :: len) :: call\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let fixture_root = dir.join("fixture");
    let assets_dir = fixture_root.join("assets");
    fs::create_dir_all(&assets_dir).expect("fixture assets dir should exist");
    let asset_path = assets_dir.join("alpha.txt");
    fs::write(&asset_path, "closure").expect("fixture asset should write");

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_std_wrapper_closure")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
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
            "import std.text\n",
            "import arcana_process.fs\n",
            "import arcana_process.io\n",
            "import arcana_process.path\n",
            "use std.result.Result\n",
            "fn unwrap_unit(result: Result[Unit, Str]) -> Bool:\n",
            "    return match result:\n",
            "        Result.Ok(_) => true\n",
            "        Result.Err(_) => false\n",
            "fn unwrap_bytes(result: Result[Bytes, Str]) -> Bytes:\n",
            "    return match result:\n",
            "        Result.Ok(value) => value\n",
            "        Result.Err(_) => std.text.bytes_from_str_utf8 :: \"\" :: call\n",
            "fn unwrap_int(result: Result[Int, Str]) -> Int:\n",
            "    return match result:\n",
            "        Result.Ok(value) => value\n",
            "        Result.Err(_) => -1\n",
            "fn main() -> Int:\n",
            "    let root = arcana_process.path.cwd :: :: call\n",
            "    let data_dir = arcana_process.path.join :: root, \"data\" :: call\n",
            "    let nested_dir = arcana_process.path.join :: data_dir, \"nested\" :: call\n",
            "    let empty_dir = arcana_process.path.join :: root, \"empty\" :: call\n",
            "    let source = arcana_process.path.join :: data_dir, \"payload.bin\" :: call\n",
            "    let copied = arcana_process.path.join :: nested_dir, \"copied.bin\" :: call\n",
            "    let moved = arcana_process.path.join :: root, \"moved.bin\" :: call\n",
            "    if not (unwrap_unit :: (arcana_process.fs.create_dir :: empty_dir :: call) :: call):\n",
            "        return 1\n",
            "    if not (unwrap_unit :: (arcana_process.fs.remove_dir :: empty_dir :: call) :: call):\n",
            "        return 2\n",
            "    if not (unwrap_unit :: (arcana_process.fs.create_dir :: data_dir :: call) :: call):\n",
            "        return 3\n",
            "    if not (unwrap_unit :: (arcana_process.fs.mkdir_all :: nested_dir :: call) :: call):\n",
            "        return 4\n",
            "    let payload = std.text.bytes_from_str_utf8 :: \"arc\" :: call\n",
            "    if not (unwrap_unit :: (arcana_process.fs.write_bytes :: source, payload :: call) :: call):\n",
            "        return 5\n",
            "    if not (unwrap_unit :: (arcana_process.fs.copy_file :: source, copied :: call) :: call):\n",
            "        return 6\n",
            "    if not (unwrap_unit :: (arcana_process.fs.rename :: copied, moved :: call) :: call):\n",
            "        return 7\n",
            "    let read_back = unwrap_bytes :: (arcana_process.fs.read_bytes :: moved :: call) :: call\n",
            "    let size = unwrap_int :: (arcana_process.fs.file_size :: moved :: call) :: call\n",
            "    let modified = unwrap_int :: (arcana_process.fs.modified_unix_ms :: moved :: call) :: call\n",
            "    arcana_process.io.print[Bool] :: (arcana_process.fs.exists :: source :: call) :: call\n",
            "    arcana_process.io.print[Str] :: (std.text.bytes_to_str_utf8 :: read_back :: call) :: call\n",
            "    arcana_process.io.print[Int] :: size :: call\n",
            "    arcana_process.io.print[Bool] :: (modified > 0) :: call\n",
            "    if not (unwrap_unit :: (arcana_process.fs.remove_file :: source :: call) :: call):\n",
            "        return 8\n",
            "    if not (unwrap_unit :: (arcana_process.fs.remove_file :: moved :: call) :: call):\n",
            "        return 9\n",
            "    if not (unwrap_unit :: (arcana_process.fs.remove_dir_all :: data_dir :: call) :: call):\n",
            "        return 10\n",
            "    arcana_process.io.print[Bool] :: (arcana_process.fs.exists :: data_dir :: call) :: call\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_std_fs_bytes")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let cwd = dir.join("fixture").to_string_lossy().replace('\\', "/");
    fs::create_dir_all(dir.join("fixture")).expect("fixture root should exist");
    let mut host = BufferedHost {
        cwd: cwd.clone(),
        sandbox_root: cwd,
        ..BufferedHost::default()
    };
    eprintln!("[asset-test] executing main");
    let code = execute_main(&plan, &mut host).expect("runtime should execute");
    eprintln!("[asset-test] main executed");

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
    let process_dep = repo_root()
        .join("grimoires")
        .join("arcana")
        .join("process")
        .to_string_lossy()
        .replace('\\', "/");
    let winapi_dep = repo_root()
        .join("grimoires")
        .join("arcana")
        .join("winapi")
        .to_string_lossy()
        .replace('\\', "/");
    write_file(
        &dir.join("book.toml"),
        &format!(
            concat!(
                "name = \"runtime_std_fs_streams\"\n",
                "kind = \"app\"\n",
                "[deps]\n",
                "arcana_process = {process_dep:?}\n",
                "arcana_winapi = {winapi_dep:?}\n",
            ),
            process_dep = process_dep,
            winapi_dep = winapi_dep,
        ),
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.text\n",
            "import arcana_process.fs\n",
            "import arcana_winapi.process_handles\n",
            "use std.result.Result\n",
            "use arcana_winapi.process_handles.FileStream\n",
            "fn write_and_close(take stream: FileStream, read bytes: Bytes) -> Int:\n",
            "    let mut stream = stream\n",
            "    let wrote = match (arcana_process.fs.stream_write :: stream, bytes :: call):\n",
            "        Result.Ok(count) => count\n",
            "        Result.Err(_) => -1\n",
            "    if wrote < 0:\n",
            "        return 1\n",
            "    if wrote != (std.text.bytes_len :: bytes :: call):\n",
            "        return 2\n",
            "    let close_result = arcana_process.fs.stream_close :: stream :: call\n",
            "    if close_result :: :: is_err:\n",
            "        return 3\n",
            "    return 0\n",
            "fn verify_read(take stream: FileStream) -> Int:\n",
            "    let mut stream = stream\n",
            "    let empty = std.text.bytes_from_str_utf8 :: \"\" :: call\n",
            "    let first_result = arcana_process.fs.stream_read :: stream, 5 :: call\n",
            "    if first_result :: :: is_err:\n",
            "        return 4\n",
            "    let first = match first_result:\n",
            "        Result.Ok(bytes) => bytes\n",
            "        Result.Err(_) => empty\n",
            "    if (std.text.bytes_to_str_utf8 :: first :: call) != \"hello\":\n",
            "        return 5\n",
            "    let before_eof_result = arcana_process.fs.stream_eof :: stream :: call\n",
            "    if before_eof_result :: :: is_err:\n",
            "        return 6\n",
            "    let before_eof = match before_eof_result:\n",
            "        Result.Ok(value) => value\n",
            "        Result.Err(_) => false\n",
            "    if before_eof:\n",
            "        return 7\n",
            "    let second_result = arcana_process.fs.stream_read :: stream, 5 :: call\n",
            "    if second_result :: :: is_err:\n",
            "        return 8\n",
            "    let second = match second_result:\n",
            "        Result.Ok(bytes) => bytes\n",
            "        Result.Err(_) => empty\n",
            "    if (std.text.bytes_to_str_utf8 :: second :: call) != \"!\":\n",
            "        return 9\n",
            "    let after_eof_result = arcana_process.fs.stream_eof :: stream :: call\n",
            "    if after_eof_result :: :: is_err:\n",
            "        return 10\n",
            "    let after_eof = match after_eof_result:\n",
            "        Result.Ok(value) => value\n",
            "        Result.Err(_) => false\n",
            "    if not after_eof:\n",
            "        return 11\n",
            "    let close_result = arcana_process.fs.stream_close :: stream :: call\n",
            "    if close_result :: :: is_err:\n",
            "        return 12\n",
            "    return 0\n",
            "fn main() -> Int:\n",
            "    let hello = std.text.bytes_from_str_utf8 :: \"hello\" :: call\n",
            "    let bang = std.text.bytes_from_str_utf8 :: \"!\" :: call\n",
            "    let write_status = match (arcana_process.fs.stream_open_write :: \"notes.bin\", false :: call):\n",
            "        Result.Ok(stream) => write_and_close :: stream, hello :: call\n",
            "        Result.Err(_) => 20\n",
            "    if write_status != 0:\n",
            "        return 21\n",
            "    let append_status = match (arcana_process.fs.stream_open_write :: \"notes.bin\", true :: call):\n",
            "        Result.Ok(stream) => write_and_close :: stream, bang :: call\n",
            "        Result.Err(_) => 22\n",
            "    if append_status != 0:\n",
            "        return 23\n",
            "    let read_status = match (arcana_process.fs.stream_open_read :: \"notes.bin\" :: call):\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_std_fs_streams")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
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
            "import arcana_process.io\n",
            "record Counter:\n",
            "    value: Int\n",
            "impl Counter:\n",
            "    fn double(read self: Counter) -> Int:\n",
            "        return self.value * 2\n",
            "fn main() -> Int:\n",
            "    let counter = Counter :: value = 7 :: call\n",
            "    arcana_process.io.print[Int] :: counter.value :: call\n",
            "    arcana_process.io.print[Int] :: (counter :: :: double) :: call\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_record_method")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(host.stdout, vec!["7".to_string(), "14".to_string()]);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_value_surface_struct_array_and_float_routines() {
    let dir = temp_workspace_dir("value_surface_runtime");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_value_surface\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import arcana_process.io\n",
            "struct Vec2:\n",
            "    x: F32\n",
            "    y: F32\n",
            "array Trio[Int, 3]:\n",
            "fn main() -> Int:\n",
            "    let point = Vec2 :: x = 1.5f32, y = 2.5f32 :: call\n",
            "    let xs = Trio :: 1, 2, 3 :: call\n",
            "    let ys = array yield Trio -return 0\n",
            "        [0] = 4\n",
            "        [1] = 5\n",
            "        [2] = 6\n",
            "    let sum = (F64 :: point.x :: call) + 2.5\n",
            "    let neg = -1.5f32\n",
            "    arcana_process.io.print[F64] :: sum :: call\n",
            "    arcana_process.io.print[F32] :: neg :: call\n",
            "    arcana_process.io.print[Int] :: xs[0] + ys[2] :: call\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_value_surface")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(
        host.stdout,
        vec!["4.0".to_string(), "-1.5".to_string(), "7".to_string()]
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_struct_bitfield_layout_semantics() {
    let dir = temp_workspace_dir("value_surface_bitfields");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_value_surface_bitfields\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import arcana_process.io\n",
            "struct Flags:\n",
            "    low: U8 bits 3\n",
            "    high: U8 bits 5\n",
            "struct SignedFlags:\n",
            "    head: I8 bits 3\n",
            "    tail: U8 bits 5\n",
            "fn main() -> Int:\n",
            "    let mut flags = Flags :: low = U8 :: 5 :: call, high = U8 :: 17 :: call :: call\n",
            "    flags.low = U8 :: 3 :: call\n",
            "    flags.high = U8 :: 9 :: call\n",
            "    let signed = SignedFlags :: head = I8 :: -1 :: call, tail = U8 :: 4 :: call :: call\n",
            "    let low = Int :: flags.low :: call\n",
            "    let high = Int :: flags.high :: call\n",
            "    let head = Int :: signed.head :: call\n",
            "    arcana_process.io.print[Int] :: low + (high * 10) :: call\n",
            "    arcana_process.io.print[Int] :: head :: call\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_value_surface_bitfields")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(host.stdout, vec!["93".to_string(), "-1".to_string()]);

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
                "import std.text\n",
                "import std.collections.list\n",
                "import arcana_process.io\n",
                "import arcana_process.process\n",
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
                "    let status = match (arcana_process.process.exec_status :: {program:?}, (status_args :: :: call) :: call):\n",
                "        Result.Ok(value) => value\n",
                "        Result.Err(_) => -1\n",
                "    let capture_result = arcana_process.process.exec_capture :: {program:?}, (capture_args :: :: call) :: call\n",
                "    if capture_result :: :: is_err:\n",
                "        return 99\n",
                "    let empty = std.text.bytes_from_str_utf8 :: \"\" :: call\n",
                "    let capture = capture_result :: (arcana_process.process.ExecCapture :: status = 0, output = (empty, empty), utf8 = (true, true) :: call) :: unwrap_or\n",
                "    let text = match (capture :: :: stdout_text):\n",
                "        Result.Ok(value) => value\n",
                "        Result.Err(_) => \"\"\n",
                "    arcana_process.io.print[Int] :: status :: call\n",
                "    arcana_process.io.print[Bool] :: (capture :: :: success) :: call\n",
                "    arcana_process.io.print[Bool] :: (std.text.starts_with :: text, \"hello\" :: call) :: call\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_std_process")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
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
            "import arcana_process.io\n",
            "use std.option.Option\n",
            "fn main() -> Int:\n",
            "    let some = Option.Some[Int] :: 5 :: call\n",
            "    let none = Option.None[Int] :: :: call\n",
            "    arcana_process.io.print[Bool] :: (some :: :: is_some) :: call\n",
            "    arcana_process.io.print[Bool] :: (none :: :: is_none) :: call\n",
            "    arcana_process.io.print[Int] :: (some :: 0 :: unwrap_or) :: call\n",
            "    arcana_process.io.print[Int] :: (none :: 9 :: unwrap_or) :: call\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_std_option")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
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
            "import arcana_process.io\n",
            "import std.text\n",
            "use std.text as texts\n",
            "fn main() -> Int:\n",
            "    arcana_process.io.print[Bool] :: (\"arcana\" :: \"arc\" :: texts.starts_with) :: call\n",
            "    arcana_process.io.print[Bool] :: (\"arcana\" :: \"ana\" :: texts.ends_with) :: call\n",
            "    arcana_process.io.print[Int] :: (\"arcana\" :: 0, \"can\" :: texts.find) :: call\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_named_qualifier_path")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
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
            "import arcana_process.io\n",
            "import std.result\n",
            "use std.result.Result\n",
            "fn main() -> Int:\n",
            "    let ok = Result.Ok[Int, Str] :: 7 :: call\n",
            "    let err = Result.Err[Int, Str] :: \"bad\" :: call\n",
            "    arcana_process.io.print[Bool] :: (ok :: :: is_ok) :: call\n",
            "    arcana_process.io.print[Bool] :: (err :: :: is_err) :: call\n",
            "    arcana_process.io.print[Int] :: (ok :: 0 :: unwrap_or) :: call\n",
            "    arcana_process.io.print[Int] :: (err :: 13 :: unwrap_or) :: call\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_std_result")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
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
            "import arcana_process.io\n",
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
            "    arcana_process.io.print[Int] :: ok_value :: call\n",
            "    arcana_process.io.print[Str] :: err_value :: call\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_try_qualifier")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(host.stdout, vec!["8".to_string(), "bad".to_string()]);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_matches_zero_payload_variant_names() {
    let dir = temp_workspace_dir("match_zero_payload_variant");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_match_zero_payload_variant\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "enum Maybe:\n",
            "    None\n",
            "    Some(Int)\n",
            "fn main() -> Int:\n",
            "    let value = Maybe.None :: :: call\n",
            "    return match value:\n",
            "        None => 7\n",
            "        Some(_) => 0\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_match_zero_payload_variant")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 7);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_preserves_uppercase_match_bindings() {
    let dir = temp_workspace_dir("match_uppercase_binding");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_match_uppercase_binding\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "fn main() -> Int:\n",
            "    return match 7:\n",
            "        Value => Value\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_match_uppercase_binding")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 7);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_rejects_try_qualifier_arguments() {
    let plan = RuntimePackagePlan {
        package_id: "try_args_runtime".to_string(),
        package_name: "try_args_runtime".to_string(),
        root_module_id: "try_args_runtime".to_string(),
        direct_deps: Vec::new(),
        direct_dep_ids: Vec::new(),
        package_display_names: test_package_display_names_with_deps(
            "try_args_runtime".to_string(),
            "try_args_runtime".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        package_direct_dep_ids: test_package_direct_dep_ids(
            "try_args_runtime".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        runtime_requirements: Vec::new(),
        foreword_index: Vec::new(),
        foreword_registrations: Vec::new(),
        module_aliases: BTreeMap::new(),
        opaque_family_types: BTreeMap::new(),
        entrypoints: vec![RuntimeEntrypointPlan {
            package_id: test_package_id_for_module("try_args_runtime"),
            module_id: "try_args_runtime".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            is_async: false,
            exported: true,
            routine_index: 0,
        }],
        native_callbacks: Vec::new(),
        shackle_decls: Vec::new(),
        binding_layouts: Vec::new(),
        owners: Vec::new(),
        routines: vec![RuntimeRoutinePlan {
            package_id: test_package_id_for_module("try_args_runtime"),
            module_id: "try_args_runtime".to_string(),
            routine_key: "try_args_runtime#sym-0".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            exported: true,
            is_async: false,
            type_params: Vec::new(),
            behavior_attrs: BTreeMap::new(),
            params: Vec::new(),
            return_type: test_return_type("fn main() -> Int:"),
            intrinsic_impl: None,
            native_impl: None,
            impl_target_type: None,
            impl_trait_path: None,
            availability: Vec::new(),
            cleanup_footers: Vec::new(),
            statements: vec![
                ParsedStmt::Let {
                    binding_id: 0,
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
                        qualifier_type_args: Vec::new(),
                        resolved_callable: None,
                        resolved_routine: None,
                        dynamic_dispatch: None,
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
                        qualifier_type_args: Vec::new(),
                        resolved_callable: None,
                        resolved_routine: None,
                        dynamic_dispatch: None,
                        attached: Vec::new(),
                    },
                    cleanup_footers: Vec::new(),
                },
                ParsedStmt::ReturnValue {
                    value: ParsedExpr::Int(0),
                },
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
            "import std.collections.set\n",
            "import arcana_process.io\n",
            "fn main() -> Int:\n",
            "    let mut xs = std.collections.list.new[Int] :: :: call\n",
            "    xs :: 4 :: push\n",
            "    xs :: 7 :: push\n",
            "    arcana_process.io.print[Int] :: (xs :: :: len) :: call\n",
            "    arcana_process.io.print[Int] :: (xs :: :: pop) :: call\n",
            "    let popped = xs :: 9 :: try_pop_or\n",
            "    arcana_process.io.print[Bool] :: popped.0 :: call\n",
            "    arcana_process.io.print[Int] :: popped.1 :: call\n",
            "    let fallback = xs :: 11 :: try_pop_or\n",
            "    arcana_process.io.print[Bool] :: fallback.0 :: call\n",
            "    arcana_process.io.print[Int] :: fallback.1 :: call\n",
            "    let arr = std.collections.array.new[Int] :: 2, 5 :: call\n",
            "    arcana_process.io.print[Int] :: ((arr :: :: to_list) :: :: len) :: call\n",
            "    let empty_arr = std.collections.array.empty[Int] :: :: call\n",
            "    arcana_process.io.print[Int] :: (empty_arr :: :: len) :: call\n",
            "    let mut drained_list_source = std.collections.list.empty[Int] :: :: call\n",
            "    drained_list_source :: 3 :: push\n",
            "    drained_list_source :: 5 :: push\n",
            "    let drained_list = drained_list_source :: :: drain\n",
            "    arcana_process.io.print[Int] :: (drained_list :: :: len) :: call\n",
            "    arcana_process.io.print[Int] :: (drained_list_source :: :: len) :: call\n",
            "    let mut mapping = std.collections.map.new[Str, Int] :: :: call\n",
            "    mapping :: \"a\", 1 :: set\n",
            "    mapping :: \"b\", 2 :: set\n",
            "    arcana_process.io.print[Int] :: (mapping :: :: len) :: call\n",
            "    let drained_map = mapping :: :: drain\n",
            "    arcana_process.io.print[Int] :: (drained_map :: :: len) :: call\n",
            "    arcana_process.io.print[Int] :: (mapping :: :: len) :: call\n",
            "    let mut seen = std.collections.set.empty[Str] :: :: call\n",
            "    let _ = seen :: \"x\" :: insert\n",
            "    let _ = seen :: \"y\" :: insert\n",
            "    let drained_set = seen :: :: drain\n",
            "    arcana_process.io.print[Int] :: (drained_set :: :: len) :: call\n",
            "    arcana_process.io.print[Int] :: (seen :: :: len) :: call\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_std_collection_methods")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
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
            "0".to_string(),
            "2".to_string(),
            "0".to_string(),
            "2".to_string(),
            "2".to_string(),
            "0".to_string(),
            "2".to_string(),
            "0".to_string(),
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
            "import arcana_process.io\n",
            "fn main() -> Int:\n",
            "    let xs = [10, 20, 30, 40]\n",
            "    arcana_process.io.print[Int] :: xs[0] :: call\n",
            "    let tail = xs[1..]\n",
            "    arcana_process.io.print[Int] :: (tail :: :: len) :: call\n",
            "    arcana_process.io.print[Int] :: tail[0] :: call\n",
            "    let mid = xs[1..=2]\n",
            "    arcana_process.io.print[Int] :: (mid :: :: len) :: call\n",
            "    arcana_process.io.print[Int] :: mid[1] :: call\n",
            "    let whole = xs[..]\n",
            "    arcana_process.io.print[Int] :: (whole :: :: len) :: call\n",
            "    let arr = std.collections.array.new[Int] :: 3, 5 :: call\n",
            "    arcana_process.io.print[Int] :: arr[1] :: call\n",
            "    arcana_process.io.print[Int] :: ((arr[1..]) :: :: len) :: call\n",
            "    let mut sum = 0\n",
            "    for i in 1..4:\n",
            "        sum = sum + i\n",
            "    arcana_process.io.print[Int] :: sum :: call\n",
            "    let r1 = 1..4\n",
            "    let r2 = 1..4\n",
            "    let r3 = ..=3\n",
            "    let r4 = ..=3\n",
            "    arcana_process.io.print[Bool] :: (r1 == r2) :: call\n",
            "    arcana_process.io.print[Bool] :: (r3 == r4) :: call\n",
            "    let as_text = match 2:\n",
            "        1 => \"one\"\n",
            "        2 => \"two\"\n",
            "        _ => \"other\"\n",
            "    arcana_process.io.print[Str] :: as_text :: call\n",
            "    let flag = match false:\n",
            "        true => \"yes\"\n",
            "        false => \"no\"\n",
            "    arcana_process.io.print[Str] :: flag :: call\n",
            "    let fruit = match \"pear\":\n",
            "        \"apple\" => \"miss\"\n",
            "        \"pear\" => \"hit\"\n",
            "        _ => \"other\"\n",
            "    arcana_process.io.print[Str] :: fruit :: call\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_range_index_slice_match")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
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
            "import arcana_process.io\n",
            "fn main() -> Int:\n",
            "    let mut xs = [1, 2, 3]\n",
            "    xs[1] = 9\n",
            "    xs[2] += 5\n",
            "    arcana_process.io.print[Int] :: xs[1] :: call\n",
            "    arcana_process.io.print[Int] :: xs[2] :: call\n",
            "    let mut arr = std.collections.array.new[Int] :: 3, 4 :: call\n",
            "    arr[0] = 7\n",
            "    arr[2] += 3\n",
            "    arcana_process.io.print[Int] :: arr[0] :: call\n",
            "    arcana_process.io.print[Int] :: arr[2] :: call\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_indexed_assignment")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
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
        package_id: "take_move_runtime".to_string(),
        package_name: "take_move_runtime".to_string(),
        root_module_id: "take_move_runtime".to_string(),
        direct_deps: Vec::new(),
        direct_dep_ids: Vec::new(),
        package_display_names: test_package_display_names_with_deps(
            "take_move_runtime".to_string(),
            "take_move_runtime".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        package_direct_dep_ids: test_package_direct_dep_ids(
            "take_move_runtime".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        runtime_requirements: Vec::new(),
        foreword_index: Vec::new(),
        foreword_registrations: Vec::new(),
        module_aliases: BTreeMap::new(),
        opaque_family_types: BTreeMap::new(),
        entrypoints: vec![RuntimeEntrypointPlan {
            package_id: test_package_id_for_module("take_move_runtime"),
            module_id: "take_move_runtime".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            is_async: false,
            exported: true,
            routine_index: 2,
        }],
        native_callbacks: Vec::new(),
        shackle_decls: Vec::new(),
        binding_layouts: Vec::new(),
        owners: Vec::new(),
        routines: vec![
            RuntimeRoutinePlan {
                package_id: test_package_id_for_module("take_move_runtime"),
                module_id: "take_move_runtime".to_string(),
                routine_key: "take_move_runtime#sym-0".to_string(),
                symbol_name: "consume".to_string(),
                symbol_kind: "fn".to_string(),
                exported: false,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: vec![RuntimeParamPlan {
                    binding_id: 0,
                    mode: Some("take".to_string()),
                    name: "value".to_string(),
                    ty: parse_routine_type_text("Str").expect("type"),
                }],
                return_type: test_return_type("fn consume(take value: Str) -> Int:"),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                cleanup_footers: Vec::new(),
                statements: vec![ParsedStmt::ReturnValue {
                    value: ParsedExpr::Int(1),
                }],
            },
            RuntimeRoutinePlan {
                package_id: test_package_id_for_module("take_move_runtime"),
                module_id: "take_move_runtime".to_string(),
                routine_key: "take_move_runtime#sym-1".to_string(),
                symbol_name: "reuse".to_string(),
                symbol_kind: "fn".to_string(),
                exported: false,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: vec![RuntimeParamPlan {
                    binding_id: 0,
                    mode: Some("read".to_string()),
                    name: "value".to_string(),
                    ty: parse_routine_type_text("Str").expect("type"),
                }],
                return_type: test_return_type("fn reuse(read value: Str) -> Int:"),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                cleanup_footers: Vec::new(),
                statements: vec![ParsedStmt::ReturnValue {
                    value: ParsedExpr::Int(0),
                }],
            },
            RuntimeRoutinePlan {
                package_id: test_package_id_for_module("take_move_runtime"),
                module_id: "take_move_runtime".to_string(),
                routine_key: "take_move_runtime#sym-2".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: Vec::new(),
                return_type: test_return_type("fn main() -> Int:"),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                cleanup_footers: Vec::new(),
                statements: vec![
                    ParsedStmt::Let {
                        binding_id: 0,
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
                            qualifier_type_args: Vec::new(),
                            resolved_callable: None,
                            resolved_routine: None,
                            dynamic_dispatch: None,
                            attached: Vec::new(),
                        },
                        cleanup_footers: Vec::new(),
                    },
                    ParsedStmt::ReturnValue {
                        value: ParsedExpr::Phrase {
                            subject: Box::new(ParsedExpr::Path(vec!["reuse".to_string()])),
                            args: vec![ParsedPhraseArg {
                                name: None,
                                value: ParsedExpr::Path(vec!["s".to_string()]),
                            }],
                            qualifier_kind: ParsedPhraseQualifierKind::Call,
                            qualifier: "call".to_string(),
                            qualifier_type_args: Vec::new(),
                            resolved_callable: None,
                            resolved_routine: None,
                            dynamic_dispatch: None,
                            attached: Vec::new(),
                        },
                    },
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
        package_id: "take_intrinsic_runtime".to_string(),
        package_name: "take_intrinsic_runtime".to_string(),
        root_module_id: "take_intrinsic_runtime".to_string(),
        direct_deps: Vec::new(),
        direct_dep_ids: Vec::new(),
        package_display_names: test_package_display_names_with_deps(
            "take_intrinsic_runtime".to_string(),
            "take_intrinsic_runtime".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        package_direct_dep_ids: test_package_direct_dep_ids(
            "take_intrinsic_runtime".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        runtime_requirements: Vec::new(),
        foreword_index: Vec::new(),
        foreword_registrations: Vec::new(),
        module_aliases: BTreeMap::new(),
        opaque_family_types: BTreeMap::new(),
        entrypoints: vec![RuntimeEntrypointPlan {
            package_id: test_package_id_for_module("take_intrinsic_runtime"),
            module_id: "take_intrinsic_runtime".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            is_async: false,
            exported: true,
            routine_index: 0,
        }],
        native_callbacks: Vec::new(),
        shackle_decls: Vec::new(),
        binding_layouts: Vec::new(),
        owners: Vec::new(),
        routines: vec![RuntimeRoutinePlan {
            package_id: test_package_id_for_module("take_intrinsic_runtime"),
            module_id: "take_intrinsic_runtime".to_string(),
            routine_key: "take_intrinsic_runtime#sym-0".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            exported: true,
            is_async: false,
            type_params: Vec::new(),
            behavior_attrs: BTreeMap::new(),
            params: Vec::new(),
            return_type: test_return_type("fn main() -> Int:"),
            intrinsic_impl: None,
            native_impl: None,
            impl_target_type: None,
            impl_trait_path: None,
            availability: Vec::new(),
            cleanup_footers: Vec::new(),
            statements: vec![
                ParsedStmt::Let {
                    binding_id: 0,
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
                        qualifier_type_args: Vec::new(),
                        resolved_callable: None,
                        resolved_routine: None,
                        dynamic_dispatch: None,
                        attached: Vec::new(),
                    },
                    cleanup_footers: Vec::new(),
                },
                ParsedStmt::ReturnValue {
                    value: ParsedExpr::Phrase {
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
                        qualifier_type_args: Vec::new(),
                        resolved_callable: None,
                        resolved_routine: None,
                        dynamic_dispatch: None,
                        attached: Vec::new(),
                    },
                },
            ],
        }],
    };
    let mut host = BufferedHost::default();
    let err = execute_main(&plan, &mut host).expect_err("runtime should reject moved-local reuse");

    assert!(err.contains("use of moved local `xs`"), "{err}");
}

#[test]
fn execute_main_binds_named_args_for_direct_intrinsic_fallback() {
    let plan = RuntimePackagePlan {
        package_id: "named_intrinsic_runtime".to_string(),
        package_name: "named_intrinsic_runtime".to_string(),
        root_module_id: "named_intrinsic_runtime".to_string(),
        direct_deps: Vec::new(),
        direct_dep_ids: Vec::new(),
        package_display_names: test_package_display_names_with_deps(
            "named_intrinsic_runtime".to_string(),
            "named_intrinsic_runtime".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        package_direct_dep_ids: test_package_direct_dep_ids(
            "named_intrinsic_runtime".to_string(),
            Vec::new(),
            Vec::new(),
        ),
        runtime_requirements: Vec::new(),
        foreword_index: Vec::new(),
        foreword_registrations: Vec::new(),
        module_aliases: BTreeMap::new(),
        opaque_family_types: BTreeMap::new(),
        entrypoints: vec![RuntimeEntrypointPlan {
            package_id: test_package_id_for_module("named_intrinsic_runtime"),
            module_id: "named_intrinsic_runtime".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            is_async: false,
            exported: true,
            routine_index: 0,
        }],
        native_callbacks: Vec::new(),
        shackle_decls: Vec::new(),
        binding_layouts: Vec::new(),
        owners: Vec::new(),
        routines: vec![RuntimeRoutinePlan {
            package_id: test_package_id_for_module("named_intrinsic_runtime"),
            module_id: "named_intrinsic_runtime".to_string(),
            routine_key: "named_intrinsic_runtime#sym-0".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            exported: true,
            is_async: false,
            type_params: Vec::new(),
            behavior_attrs: BTreeMap::new(),
            params: Vec::new(),
            return_type: test_return_type("fn main() -> Int:"),
            intrinsic_impl: None,
            native_impl: None,
            impl_target_type: None,
            impl_trait_path: None,
            availability: Vec::new(),
            cleanup_footers: Vec::new(),
            statements: vec![
                ParsedStmt::Let {
                    binding_id: 0,
                    mutable: true,
                    name: "xs".to_string(),
                    value: ParsedExpr::Collection { items: Vec::new() },
                },
                ParsedStmt::Expr {
                    expr: ParsedExpr::Phrase {
                        subject: Box::new(ParsedExpr::Path(vec![
                            "std".to_string(),
                            "kernel".to_string(),
                            "collections".to_string(),
                            "list_push".to_string(),
                        ])),
                        args: vec![
                            ParsedPhraseArg {
                                name: Some("value".to_string()),
                                value: ParsedExpr::Int(7),
                            },
                            ParsedPhraseArg {
                                name: Some("xs".to_string()),
                                value: ParsedExpr::Path(vec!["xs".to_string()]),
                            },
                        ],
                        qualifier_kind: ParsedPhraseQualifierKind::Call,
                        qualifier: "call".to_string(),
                        qualifier_type_args: vec!["Int".to_string()],
                        resolved_callable: None,
                        resolved_routine: None,
                        dynamic_dispatch: None,
                        attached: Vec::new(),
                    },
                    cleanup_footers: Vec::new(),
                },
                ParsedStmt::ReturnValue {
                    value: ParsedExpr::Phrase {
                        subject: Box::new(ParsedExpr::Path(vec![
                            "std".to_string(),
                            "kernel".to_string(),
                            "collections".to_string(),
                            "list_pop".to_string(),
                        ])),
                        args: vec![ParsedPhraseArg {
                            name: None,
                            value: ParsedExpr::Path(vec!["xs".to_string()]),
                        }],
                        qualifier_kind: ParsedPhraseQualifierKind::Call,
                        qualifier: "call".to_string(),
                        qualifier_type_args: vec!["Int".to_string()],
                        resolved_callable: None,
                        resolved_routine: None,
                        dynamic_dispatch: None,
                        attached: Vec::new(),
                    },
                },
            ],
        }],
    };
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should bind direct intrinsic args");

    assert_eq!(code, 7);
}

#[test]
fn execute_main_allows_zero_arg_result_ok_direct_intrinsic_fallback() {
    let dir = temp_workspace_dir("result_ok_unit_direct_intrinsic");
    write_file(
        &dir.join("book.toml"),
        "name = \"result_ok_unit_direct_intrinsic\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.result\n",
            "use std.result.Result\n",
            "fn main() -> Int:\n",
            "    let ok = Result.Ok[Unit, Str] :: :: call\n",
            "    return match ok:\n",
            "        Result.Ok(_) => 0\n",
            "        Result.Err(_) => 1\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let plan = build_workspace_plan_for_member(&dir, "result_ok_unit_direct_intrinsic");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host)
        .expect("runtime should allow zero-arg Result.Ok direct intrinsic fallback");

    assert_eq!(code, 0);

    let _ = fs::remove_dir_all(dir);
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
            "import arcana_process.io\n",
            "fn consume_text(take value: Str):\n",
            "    return\n",
            "fn consume_int(take value: Int) -> Int:\n",
            "    return value + 1\n",
            "fn main() -> Int:\n",
            "    let mut s = \"hi\"\n",
            "    consume_text :: s :: call\n",
            "    s = \"bye\"\n",
            "    arcana_process.io.print[Str] :: s :: call\n",
            "    let x = 4\n",
            "    arcana_process.io.print[Int] :: (consume_int :: x :: call) :: call\n",
            "    arcana_process.io.print[Int] :: x :: call\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_take_copy_and_reassign")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
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
            "import arcana_process.io\n",
            "fn add(left: Int, right: Int) -> Int:\n",
            "    return left + right\n",
            "async fn compute(value: Int) -> Int:\n",
            "    return value + 2\n",
            "fn main() -> Int:\n",
            "    arcana_process.io.print[Int] :: (add :: 2, 3 :: >) :: call\n",
            "    let task = weave 7\n",
            "    arcana_process.io.print[Int] :: (task :: :: >>) :: call\n",
            "    arcana_process.io.print[Int] :: (compute :: 5 :: >>) :: call\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_apply_and_await_apply")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
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
fn execute_main_runs_phrase_await_weave_split_must_and_fallback_qualifiers() {
    let dir = temp_workspace_dir("phrase_runtime_qualifiers");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_phrase_qualifiers\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import arcana_process.io\n",
            "import std.option\n",
            "import std.result\n",
            "use std.option.Option\n",
            "use std.result.Result\n",
            "async fn worker(value: Int) -> Int:\n",
            "    return value + 1\n",
            "fn helper(value: Int) -> Int:\n",
            "    return value * 2\n",
            "fn main() -> Int:\n",
            "    let awaited_task = ((worker :: 4 :: weave) :: :: await)\n",
            "    let awaited_thread = ((helper :: 5 :: split) :: :: await)\n",
            "    let must_value = ((Result.Ok[Int, Str] :: 6 :: call) :: :: must)\n",
            "    let fallback_value = ((Option.None[Int] :: :: call) :: 7 :: fallback)\n",
            "    arcana_process.io.print[Int] :: awaited_task :: call\n",
            "    arcana_process.io.print[Int] :: awaited_thread :: call\n",
            "    arcana_process.io.print[Int] :: must_value :: call\n",
            "    arcana_process.io.print[Int] :: fallback_value :: call\n",
            "    return 0\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let plan = build_workspace_plan_for_member(&dir, "runtime_phrase_qualifiers");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(
        host.stdout,
        vec![
            "5".to_string(),
            "10".to_string(),
            "6".to_string(),
            "7".to_string(),
        ]
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
            "import arcana_process.io\n",
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
            "    arcana_process.io.print[Int] :: (std.ecs.get_component[Int] :: :: call) :: call\n",
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_std_ecs")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
    );
    let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute");

    assert_eq!(code, 0);
    assert_eq!(host.stdout, vec!["18".to_string()]);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn resolve_routine_index_uses_current_package_dep_id_when_display_names_collide() {
    let app_v1 = "path:app_v1".to_string();
    let app_v2 = "path:app_v2".to_string();
    let core_v1 = "registry:local:core@1.0.0".to_string();
    let core_v2 = "registry:local:core@2.0.0".to_string();
    let callable = vec!["core".to_string(), "value".to_string()];
    let plan = RuntimePackagePlan {
        package_id: app_v1.clone(),
        package_name: "app_v1".to_string(),
        root_module_id: "app_v1".to_string(),
        direct_deps: vec!["core".to_string()],
        direct_dep_ids: vec![core_v1.clone()],
        package_display_names: BTreeMap::from([
            (app_v1.clone(), "app_v1".to_string()),
            (app_v2.clone(), "app_v2".to_string()),
            (core_v1.clone(), "core".to_string()),
            (core_v2.clone(), "core".to_string()),
        ]),
        package_direct_dep_ids: BTreeMap::from([
            (
                app_v1.clone(),
                BTreeMap::from([("core".to_string(), core_v1.clone())]),
            ),
            (
                app_v2.clone(),
                BTreeMap::from([("core".to_string(), core_v2.clone())]),
            ),
            (core_v1.clone(), BTreeMap::new()),
            (core_v2.clone(), BTreeMap::new()),
        ]),
        runtime_requirements: Vec::new(),
        foreword_index: Vec::new(),
        foreword_registrations: Vec::new(),
        module_aliases: BTreeMap::new(),
        opaque_family_types: BTreeMap::new(),
        entrypoints: Vec::new(),
        native_callbacks: Vec::new(),
        shackle_decls: Vec::new(),
        binding_layouts: Vec::new(),
        owners: Vec::new(),
        routines: vec![
            RuntimeRoutinePlan {
                package_id: core_v1.clone(),
                module_id: "core".to_string(),
                routine_key: "core@1#fn-0".to_string(),
                symbol_name: "value".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: Vec::new(),
                return_type: test_return_type("fn value() -> Int:"),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                cleanup_footers: Vec::new(),
                statements: Vec::new(),
            },
            RuntimeRoutinePlan {
                package_id: core_v2.clone(),
                module_id: "core".to_string(),
                routine_key: "core@2#fn-0".to_string(),
                symbol_name: "value".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: Vec::new(),
                return_type: test_return_type("fn value() -> Int:"),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                cleanup_footers: Vec::new(),
                statements: Vec::new(),
            },
        ],
    };

    let from_app_v1 = resolve_routine_index(&plan, &app_v1, "app_v1", &callable)
        .expect("app_v1 should resolve its direct core dependency");
    let from_app_v2 = resolve_routine_index(&plan, &app_v2, "app_v2", &callable)
        .expect("app_v2 should resolve its direct core dependency");

    assert_eq!(plan.routines[from_app_v1].package_id, core_v1);
    assert_eq!(plan.routines[from_app_v2].package_id, core_v2);
}

#[test]
fn resolve_routine_index_rejects_globally_unique_package_name_without_direct_dep_visibility() {
    let app = "path:app".to_string();
    let helper = "path:helper".to_string();
    let core = "registry:local:core@1.0.0".to_string();
    let callable = vec!["core".to_string(), "value".to_string()];
    let plan = RuntimePackagePlan {
        package_id: app.clone(),
        package_name: "app".to_string(),
        root_module_id: "app".to_string(),
        direct_deps: vec!["helper".to_string()],
        direct_dep_ids: vec![helper.clone()],
        package_display_names: BTreeMap::from([
            (app.clone(), "app".to_string()),
            (helper.clone(), "helper".to_string()),
            (core.clone(), "core".to_string()),
        ]),
        package_direct_dep_ids: BTreeMap::from([
            (
                app.clone(),
                BTreeMap::from([("helper".to_string(), helper.clone())]),
            ),
            (helper.clone(), BTreeMap::new()),
            (core.clone(), BTreeMap::new()),
        ]),
        runtime_requirements: Vec::new(),
        foreword_index: Vec::new(),
        foreword_registrations: Vec::new(),
        module_aliases: BTreeMap::new(),
        opaque_family_types: BTreeMap::new(),
        entrypoints: Vec::new(),
        native_callbacks: Vec::new(),
        shackle_decls: Vec::new(),
        binding_layouts: Vec::new(),
        owners: Vec::new(),
        routines: vec![RuntimeRoutinePlan {
            package_id: core,
            module_id: "core".to_string(),
            routine_key: "core@1#fn-0".to_string(),
            symbol_name: "value".to_string(),
            symbol_kind: "fn".to_string(),
            exported: true,
            is_async: false,
            type_params: Vec::new(),
            behavior_attrs: BTreeMap::new(),
            params: Vec::new(),
            return_type: test_return_type("fn value() -> Int:"),
            intrinsic_impl: None,
            native_impl: None,
            impl_target_type: None,
            impl_trait_path: None,
            availability: Vec::new(),
            cleanup_footers: Vec::new(),
            statements: Vec::new(),
        }],
    };

    let resolved = resolve_routine_index(&plan, &app, "app", &callable);
    assert!(resolved.is_none());
}

#[test]
fn plan_from_artifact_keeps_owner_package_ids_distinct_when_display_names_collide() {
    let core_v1 = "registry:local:core@1.0.0".to_string();
    let core_v2 = "registry:local:core@2.0.0".to_string();
    let owner_path = vec!["core".to_string(), "Session".to_string()];
    let mut artifact = sample_return_artifact();
    artifact.owners = vec![
        AotOwnerArtifact {
            package_id: core_v1.clone(),
            module_id: "core".to_string(),
            owner_path: owner_path.clone(),
            owner_name: "Session".to_string(),
            context_type: None,
            objects: Vec::new(),
            exits: Vec::new(),
        },
        AotOwnerArtifact {
            package_id: core_v2.clone(),
            module_id: "core".to_string(),
            owner_path: owner_path.clone(),
            owner_name: "Session".to_string(),
            context_type: None,
            objects: Vec::new(),
            exits: Vec::new(),
        },
    ];

    let plan = plan_from_artifact(&artifact).expect("runtime plan should build");
    let key_v1 = owner_state_key(&core_v1, &owner_path);
    let key_v2 = owner_state_key(&core_v2, &owner_path);

    assert_ne!(key_v1, key_v2);
    assert_eq!(
        lookup_runtime_owner_plan(&plan, &core_v1, &owner_path)
            .map(|owner| owner.package_id.as_str()),
        Some(core_v1.as_str())
    );
    assert_eq!(
        lookup_runtime_owner_plan(&plan, &core_v2, &owner_path)
            .map(|owner| owner.package_id.as_str()),
        Some(core_v2.as_str())
    );
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
    execute_workspace_build(&graph, &fingerprints, &statuses);

    let artifact_path = graph.root_dir.join(
        statuses
            .iter()
            .find(|status| status.member_name() == "runtime_host_core")
            .expect("app artifact status should exist")
            .artifact_rel_path(),
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

#[test]
fn execute_main_runs_object_owner_hold_workspace_artifact() {
    let dir = temp_workspace_dir("owner_hold");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_owner_hold\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "obj Counter:\n",
            "    value: Int\n",
            "\n",
            "create Session [Counter] scope-exit:\n",
            "    done: when Counter.value >= 10 retain [Counter]\n",
            "\n",
            "Session\n",
            "Counter\n",
            "fn main() -> Int:\n",
            "    let active = Session :: :: call\n",
            "    Counter.value = 9\n",
            "    Counter.value += 1\n",
            "    let resumed = Session :: :: call\n",
            "    return resumed.Counter.value\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let plan = build_workspace_plan_for_member(&dir, "runtime_owner_hold");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute owner hold flow");

    assert_eq!(code, 10);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_owner_multi_exit_uses_first_matching_exit() {
    let dir = temp_workspace_dir("owner_multi_exit_source_order");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_owner_multi_exit_source_order\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "obj Counter:\n",
            "    value: Int\n",
            "\n",
            "create Session [Counter] scope-exit:\n",
            "    keep: when true retain [Counter]\n",
            "    drop: when true\n",
            "\n",
            "Session\n",
            "Counter\n",
            "fn main() -> Int:\n",
            "    Session :: :: call\n",
            "    Counter.value = 7\n",
            "    let resumed = Session :: :: call\n",
            "    return resumed.Counter.value\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let plan = build_workspace_plan_for_member(&dir, "runtime_owner_multi_exit_source_order");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host)
        .expect("runtime should keep state from the first matching owner exit");

    assert_eq!(code, 7);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_rejects_stale_owner_access_after_exit() {
    let dir = temp_workspace_dir("owner_stale_after_exit");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_owner_stale_after_exit\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "obj Counter:\n",
            "    value: Int\n",
            "\n",
            "create Session [Counter] scope-exit:\n",
            "    done: when Counter.value >= 1\n",
            "\n",
            "Session\n",
            "Counter\n",
            "fn main() -> Int:\n",
            "    let active = Session :: :: call\n",
            "    Counter.value = 1\n",
            "    return active.Counter.value\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let plan = build_workspace_plan_for_member(&dir, "runtime_owner_stale_after_exit");
    let mut host = BufferedHost::default();
    let err = execute_main(&plan, &mut host).expect_err("stale owner access should fail");

    assert!(
        err.contains("explicit re-entry is required"),
        "expected explicit re-entry diagnostic, got: {err}"
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_reentry_can_exit_owner_without_realized_objects() {
    let dir = temp_workspace_dir("owner_reentry_without_objects");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_owner_reentry_without_objects\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.concurrent\n",
            "\n",
            "obj Counter:\n",
            "    value: Int\n",
            "    fn init(edit self: Self):\n",
            "        self.value = 9\n",
            "\n",
            "obj GateState:\n",
            "    gate: AtomicBool\n",
            "\n",
            "create Session [Counter, GateState] scope-exit:\n",
            "    closed: when (std.kernel.concurrency.atomic_bool_load :: GateState.gate :: call)\n",
            "\n",
            "Session\n",
            "Counter\n",
            "GateState\n",
            "fn main() -> Int:\n",
            "    Session :: :: call\n",
            "    let gate = std.concurrent.atomic_bool :: false :: call\n",
            "    GateState.gate = gate\n",
            "    gate :: true :: store\n",
            "    let resumed = Session :: :: call\n",
            "    let new_gate = std.concurrent.atomic_bool :: false :: call\n",
            "    GateState.gate = new_gate\n",
            "    return resumed.Counter.value\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let plan = build_workspace_plan_for_member(&dir, "runtime_owner_reentry_without_objects");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host)
        .expect("re-entry should restart a fresh activation even before object realization");

    assert_eq!(code, 9);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_owner_init_hook_with_activation_context() {
    let dir = temp_workspace_dir("owner_init_context");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_owner_init_context\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "obj SessionCtx:\n",
            "    base: Int\n",
            "\n",
            "obj Counter:\n",
            "    value: Int\n",
            "    fn init(edit self: Self, read ctx: SessionCtx):\n",
            "        self.value = ctx.base\n",
            "\n",
            "create Session [Counter] scope-exit:\n",
            "    done: when Counter.value > 10 retain [Counter]\n",
            "\n",
            "Session\n",
            "Counter\n",
            "fn main() -> Int:\n",
            "    let ctx = SessionCtx :: base = 4 :: call\n",
            "    Session :: ctx :: call\n",
            "    return Counter.value\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let plan = build_workspace_plan_for_member(&dir, "runtime_owner_init_context");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute owner init hook");

    assert_eq!(code, 4);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_owner_resume_hook_with_activation_context() {
    let dir = temp_workspace_dir("owner_resume_context");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_owner_resume_context\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "obj SessionCtx:\n",
            "    base: Int\n",
            "\n",
            "obj Counter:\n",
            "    value: Int\n",
            "    fn init(edit self: Self, read ctx: SessionCtx):\n",
            "        self.value = ctx.base\n",
            "    fn resume(edit self: Self, read ctx: SessionCtx):\n",
            "        self.value += ctx.base\n",
            "\n",
            "create Session [Counter] scope-exit:\n",
            "    done: when Counter.value == 3 retain [Counter]\n",
            "\n",
            "Session\n",
            "Counter\n",
            "fn main() -> Int:\n",
            "    let start = SessionCtx :: base = 1 :: call\n",
            "    Session :: start :: call\n",
            "    let first = Counter.value\n",
            "    Counter.value = 3\n",
            "    let resume_ctx = SessionCtx :: base = 2 :: call\n",
            "    let resumed = Session :: resume_ctx :: call\n",
            "    return resumed.Counter.value\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let plan = build_workspace_plan_for_member(&dir, "runtime_owner_resume_context");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("runtime should execute owner resume hook");

    assert_eq!(code, 5);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_reentry_while_active_uses_new_activation_context() {
    let dir = temp_workspace_dir("owner_reentry_while_active_context");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_owner_reentry_while_active_context\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "import std.concurrent\n",
            "\n",
            "obj SessionCtx:\n",
            "    base: Int\n",
            "\n",
            "obj Counter:\n",
            "    value: Int\n",
            "    fn init(edit self: Self, read ctx: SessionCtx):\n",
            "        self.value = ctx.base\n",
            "\n",
            "obj GateState:\n",
            "    gate: AtomicBool\n",
            "\n",
            "create Session [Counter, GateState] scope-exit:\n",
            "    closed: when (std.kernel.concurrency.atomic_bool_load :: GateState.gate :: call)\n",
            "\n",
            "Session\n",
            "Counter\n",
            "GateState\n",
            "fn main() -> Int:\n",
            "    let start = SessionCtx :: base = 1 :: call\n",
            "    Session :: start :: call\n",
            "    let gate = std.concurrent.atomic_bool :: false :: call\n",
            "    GateState.gate = gate\n",
            "    gate :: true :: store\n",
            "    let resume_ctx = SessionCtx :: base = 2 :: call\n",
            "    let resumed = Session :: resume_ctx :: call\n",
            "    let new_gate = std.concurrent.atomic_bool :: false :: call\n",
            "    GateState.gate = new_gate\n",
            "    return resumed.Counter.value\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let plan = build_workspace_plan_for_member(&dir, "runtime_owner_reentry_while_active_context");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host)
        .expect("runtime should resume with the new activation context");

    assert_eq!(code, 2);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_attached_owner_helper_with_active_state() {
    let dir = temp_workspace_dir("owner_attached_helper");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_owner_attached_helper\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "obj Counter:\n",
            "    value: Int\n",
            "\n",
            "create Session [Counter] scope-exit:\n",
            "    done: when Counter.value >= 10 retain [Counter]\n",
            "\n",
            "Session\n",
            "Counter\n",
            "fn bump() -> Int:\n",
            "    Counter.value += 1\n",
            "    return Counter.value\n",
            "\n",
            "Session\n",
            "Counter\n",
            "fn main() -> Int:\n",
            "    Session :: :: call\n",
            "    Counter.value = 4\n",
            "    return bump :: :: call\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let plan = build_workspace_plan_for_member(&dir, "runtime_owner_attached_helper");
    let mut host = BufferedHost::default();
    let code =
        execute_main(&plan, &mut host).expect("runtime should execute attached owner helper");

    assert_eq!(code, 5);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_object_only_attached_helper_with_active_state() {
    let dir = temp_workspace_dir("owner_object_only_helper");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_owner_object_only_helper\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "obj Counter:\n",
            "    value: Int\n",
            "\n",
            "create Session [Counter] scope-exit:\n",
            "    done: when Counter.value >= 10 retain [Counter]\n",
            "\n",
            "Counter\n",
            "fn bump() -> Int:\n",
            "    Counter.value += 1\n",
            "    return Counter.value\n",
            "\n",
            "Session\n",
            "Counter\n",
            "fn main() -> Int:\n",
            "    Session :: :: call\n",
            "    Counter.value = 4\n",
            "    return bump :: :: call\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let plan = build_workspace_plan_for_member(&dir, "runtime_owner_object_only_helper");
    let mut host = BufferedHost::default();
    let code =
        execute_main(&plan, &mut host).expect("runtime should execute object-only attached helper");

    assert_eq!(code, 5);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_object_only_attached_helper_through_unattached_helper_chain() {
    let dir = temp_workspace_dir("owner_object_only_helper_chain");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_owner_object_only_helper_chain\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "obj Counter:\n",
            "    value: Int\n",
            "\n",
            "create Session [Counter] scope-exit:\n",
            "    done: when Counter.value >= 10 retain [Counter]\n",
            "\n",
            "Counter\n",
            "fn bump_inner() -> Int:\n",
            "    Counter.value += 1\n",
            "    return Counter.value\n",
            "\n",
            "fn bump_middle() -> Int:\n",
            "    return bump_inner :: :: call\n",
            "\n",
            "Session\n",
            "Counter\n",
            "fn main() -> Int:\n",
            "    Session :: :: call\n",
            "    Counter.value = 4\n",
            "    return bump_middle :: :: call\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let plan = build_workspace_plan_for_member(&dir, "runtime_owner_object_only_helper_chain");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host)
        .expect("runtime should carry active owner state through unattached helper chains");

    assert_eq!(code, 5);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_nested_object_only_attached_helpers_with_active_state() {
    let dir = temp_workspace_dir("owner_nested_object_only_helper");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_owner_nested_object_only_helper\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "obj Counter:\n",
            "    value: Int\n",
            "\n",
            "create Session [Counter] scope-exit:\n",
            "    done: when Counter.value >= 10 retain [Counter]\n",
            "\n",
            "Counter\n",
            "fn bump() -> Int:\n",
            "    Counter.value += 1\n",
            "    return Counter.value\n",
            "\n",
            "Counter\n",
            "fn nested_bump() -> Int:\n",
            "    return bump :: :: call\n",
            "\n",
            "Session\n",
            "Counter\n",
            "fn main() -> Int:\n",
            "    Session :: :: call\n",
            "    Counter.value = 4\n",
            "    return nested_bump :: :: call\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let plan = build_workspace_plan_for_member(&dir, "runtime_owner_nested_object_only_helper");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host)
        .expect("runtime should execute nested object-only attached helpers");

    assert_eq!(code, 5);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_late_attached_owner_block_with_active_state() {
    let dir = temp_workspace_dir("owner_late_attached_block");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_owner_late_attached_block\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "obj Counter:\n",
            "    value: Int\n",
            "\n",
            "create Session [Counter] scope-exit:\n",
            "    done: when Counter.value >= 10 retain [Counter]\n",
            "\n",
            "Session\n",
            "fn main() -> Int:\n",
            "    Session :: :: call\n",
            "    Session\n",
            "    Counter\n",
            "    if true:\n",
            "        Counter.value = 7\n",
            "        return Counter.value\n",
            "    return 0\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let plan = build_workspace_plan_for_member(&dir, "runtime_owner_late_attached_block");
    let mut host = BufferedHost::default();
    let code =
        execute_main(&plan, &mut host).expect("runtime should execute late attached owner block");

    assert_eq!(code, 7);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_object_only_late_attached_block_with_active_state() {
    let dir = temp_workspace_dir("owner_object_only_late_attached_block");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_owner_object_only_late_attached_block\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "obj Counter:\n",
            "    value: Int\n",
            "\n",
            "create Session [Counter] scope-exit:\n",
            "    done: when Counter.value >= 10 retain [Counter]\n",
            "\n",
            "Session\n",
            "fn main() -> Int:\n",
            "    Session :: :: call\n",
            "    Counter\n",
            "    if true:\n",
            "        Counter.value = 7\n",
            "        return Counter.value\n",
            "    return 0\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let plan =
        build_workspace_plan_for_member(&dir, "runtime_owner_object_only_late_attached_block");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host)
        .expect("runtime should execute object-only late attached block");

    assert_eq!(code, 7);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_rejects_owner_object_init_without_required_context() {
    let dir = temp_workspace_dir("owner_missing_context");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_owner_missing_context\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "obj SessionCtx:\n",
            "    base: Int\n",
            "\n",
            "obj Counter:\n",
            "    value: Int\n",
            "    fn init(edit self: Self, read ctx: SessionCtx):\n",
            "        self.value = ctx.base\n",
            "\n",
            "create Session [Counter] scope-exit:\n",
            "    done: when false retain [Counter]\n",
            "\n",
            "Session\n",
            "Counter\n",
            "fn main() -> Int:\n",
            "    Session :: :: call\n",
            "    return Counter.value\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let plan = build_workspace_plan_for_member(&dir, "runtime_owner_missing_context");
    let mut host = BufferedHost::default();
    let err =
        execute_main(&plan, &mut host).expect_err("owner object init without context should fail");

    assert!(err.contains("requires an activation context"), "{err}");

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execute_main_runs_owner_activation_with_explicit_context_clause() {
    let dir = temp_workspace_dir("owner_explicit_context");
    write_file(
        &dir.join("book.toml"),
        "name = \"runtime_owner_explicit_context\"\nkind = \"app\"\n",
    );
    write_file(
        &dir.join("src").join("shelf.arc"),
        concat!(
            "obj SessionCtx:\n",
            "    base: Int\n",
            "\n",
            "obj Counter:\n",
            "    value: Int\n",
            "    fn init(edit self: Self, read ctx: SessionCtx):\n",
            "        self.value = ctx.base\n",
            "\n",
            "create Session [Counter] context: SessionCtx scope-exit:\n",
            "    done: when false retain [Counter]\n",
            "\n",
            "Session\n",
            "Counter\n",
            "fn main() -> Int:\n",
            "    let ctx = SessionCtx :: base = 12 :: call\n",
            "    Session :: ctx :: call\n",
            "    return Counter.value\n",
        ),
    );
    write_file(&dir.join("src").join("types.arc"), "// test types\n");

    let plan = build_workspace_plan_for_member(&dir, "runtime_owner_explicit_context");
    let mut host = BufferedHost::default();
    let code = execute_main(&plan, &mut host).expect("owner activation should execute");

    assert_eq!(code, 12);

    let _ = fs::remove_dir_all(dir);
}
