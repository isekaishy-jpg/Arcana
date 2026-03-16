use std::path::PathBuf;

use arcana_frontend::check_workspace_graph;
use arcana_package::{
    BuildTarget, DistributionBundle, GrimoireKind, WorkspaceGraph, default_distribution_dir,
    execute_build_with_context, load_workspace_graph, plan_build_for_target_with_context,
    plan_workspace, prepare_build_from_workspace, read_lockfile, stage_distribution_bundle,
    write_lockfile,
};

use crate::build_context::build_execution_context_for_target;

pub(crate) fn package_workspace(
    workspace_dir: PathBuf,
    target: BuildTarget,
    member: Option<String>,
    out_dir: Option<PathBuf>,
) -> Result<DistributionBundle, String> {
    let graph = load_workspace_graph(&workspace_dir)?;
    let packaged_member = resolve_package_member(&graph, member.as_deref())?;
    target.artifact_file_name(&packaged_member.kind)?;
    let packaged_member_name = packaged_member.name.clone();
    let output_dir =
        out_dir.unwrap_or_else(|| default_distribution_dir(&graph, &packaged_member_name, &target));

    let order = plan_workspace(&graph)?;
    let checked = check_workspace_graph(&graph)?;
    let (workspace, resolved_workspace) = checked.into_workspace_parts();
    let prepared = prepare_build_from_workspace(&graph, workspace, resolved_workspace)?;
    let lock_path = graph.root_dir.join("Arcana.lock");
    let existing_lock = read_lockfile(&lock_path)?;
    let execution_context = build_execution_context_for_target(&target)?;
    let statuses = plan_build_for_target_with_context(
        &graph,
        &order,
        &prepared,
        existing_lock.as_ref(),
        target.clone(),
        &execution_context,
    )?;
    execute_build_with_context(&graph, &prepared, &statuses, &execution_context)?;
    write_lockfile(&graph, &order, &statuses)?;
    stage_distribution_bundle(
        &graph,
        &statuses,
        &packaged_member_name,
        &target,
        &output_dir,
    )
}

fn resolve_package_member<'a>(
    graph: &'a WorkspaceGraph,
    requested_member: Option<&str>,
) -> Result<&'a arcana_package::WorkspaceMember, String> {
    match requested_member {
        Some(name) => graph
            .member(name)
            .ok_or_else(|| format!("workspace has no member `{name}`")),
        None => default_package_member(graph),
    }
}

fn default_package_member(
    graph: &WorkspaceGraph,
) -> Result<&arcana_package::WorkspaceMember, String> {
    if let Some(root_member) = graph.member(&graph.root_name) {
        return Ok(root_member);
    }
    match graph.members.as_slice() {
        [member] => Ok(member),
        [] => Err("workspace has no package members".to_string()),
        _ => {
            let app_members = graph
                .members
                .iter()
                .filter(|member| member.kind == GrimoireKind::App)
                .collect::<Vec<_>>();
            match app_members.as_slice() {
                [member] => Ok(*member),
                _ => Err(
                    "workspace has multiple package members; pass `--member <name>`".to_string(),
                ),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[cfg(windows)]
    use libloading::Library;
    #[cfg(windows)]
    use std::process::Command;

    use super::*;

    fn temp_dir(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("arcana_cli_package_{label}_{unique}"));
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

    fn write_lib_workspace(dir: &Path, body: &str) {
        write_file(&dir.join("book.toml"), "name = \"core\"\nkind = \"lib\"\n");
        write_file(&dir.join("src/book.arc"), body);
        write_file(&dir.join("src/types.arc"), "// types\n");
    }

    fn write_std_bytes_grimoire(dir: &Path) {
        write_file(
            &dir.join("std/book.toml"),
            "name = \"std\"\nkind = \"lib\"\n",
        );
        write_file(
            &dir.join("std/src/book.arc"),
            "import bytes\nimport kernel.text\n",
        );
        write_file(&dir.join("std/src/types.arc"), "// std types\n");
        write_file(
            &dir.join("std/src/bytes.arc"),
            concat!(
                "import std.kernel.text\n",
                "export fn from_str_utf8(text: Str) -> Array[Int]:\n",
                "    return std.kernel.text.bytes_from_str_utf8 :: text :: call\n",
                "export fn len(read bytes: Array[Int]) -> Int:\n",
                "    return std.kernel.text.bytes_len :: bytes :: call\n",
            ),
        );
        write_file(
            &dir.join("std/src/kernel/text.arc"),
            concat!(
                "intrinsic fn bytes_from_str_utf8(text: Str) -> Array[Int] = HostBytesFromStrUtf8\n",
                "intrinsic fn bytes_len(read bytes: Array[Int]) -> Int = HostBytesLen\n",
            ),
        );
    }

    #[cfg(windows)]
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct ArcanaStrView {
        ptr: *const u8,
        len: usize,
    }

    #[cfg(windows)]
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct ArcanaBytesView {
        ptr: *const u8,
        len: usize,
    }

    #[cfg(windows)]
    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct ArcanaOwnedStr {
        ptr: *mut u8,
        len: usize,
    }

    #[cfg(windows)]
    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct ArcanaOwnedBytes {
        ptr: *mut u8,
        len: usize,
    }

    #[cfg(windows)]
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct ArcanaPairView__Pair__Str__Int {
        left: ArcanaStrView,
        right: i64,
    }

    #[cfg(windows)]
    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct ArcanaPairOwned__Pair__Str__Int {
        left: ArcanaOwnedStr,
        right: i64,
    }

    #[cfg(windows)]
    #[test]
    fn package_workspace_stages_runnable_windows_exe_bundle() {
        let dir = temp_dir("windows_exe");
        write_app_workspace(&dir, "fn main() -> Int:\n    return 9\n");

        let bundle = package_workspace(dir.clone(), BuildTarget::windows_exe(), None, None)
            .expect("package should succeed");
        let exe_path = bundle.bundle_dir.join(&bundle.root_artifact);
        let status = Command::new(&exe_path)
            .arg("alpha")
            .status()
            .expect("staged bundle should launch");
        assert_eq!(status.code(), Some(9));
        assert!(bundle.manifest_path.is_file());
        assert_eq!(
            bundle.support_files,
            vec!["app.exe.arcana-native.toml".to_string()]
        );
        assert!(
            bundle
                .bundle_dir
                .join("app.exe.arcana-native.toml")
                .is_file(),
            "expected staged exe native manifest"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn package_workspace_stages_loadable_windows_dll_bundle() {
        let dir = temp_dir("windows_dll");
        write_std_bytes_grimoire(&dir);
        write_lib_workspace(
            &dir,
            concat!(
                "import std.bytes\n",
                "export fn answer() -> Int:\n",
                "    return 11\n",
                "export fn greet(read name: Str) -> Str:\n",
                "    return \"hello \" + name\n",
                "export fn prefix() -> Array[Int]:\n",
                "    return std.bytes.from_str_utf8 :: \"arc\" :: call\n",
                "export fn byte_len(read bytes: Array[Int]) -> Int:\n",
                "    return std.bytes.len :: bytes :: call\n",
                "export fn echo_pair(read pair: (Str, Int)) -> (Str, Int):\n",
                "    return pair\n",
            ),
        );

        let bundle = package_workspace(dir.clone(), BuildTarget::windows_dll(), None, None)
            .expect("dll package should succeed");
        let dll_path = bundle.bundle_dir.join(&bundle.root_artifact);
        assert!(
            dll_path.is_file(),
            "expected staged dll at {}",
            dll_path.display()
        );
        assert_eq!(
            bundle.support_files,
            vec![
                "lib.dll.h".to_string(),
                "lib.dll.def".to_string(),
                "lib.dll.arcana-native.toml".to_string()
            ]
        );
        assert!(
            bundle.bundle_dir.join("lib.dll.def").is_file(),
            "expected staged dll definition file"
        );
        assert!(
            bundle
                .bundle_dir
                .join("lib.dll.arcana-native.toml")
                .is_file(),
            "expected staged dll native manifest"
        );

        unsafe {
            let library = Library::new(&dll_path).expect("dll should load");
            let answer = library
                .get::<unsafe extern "system" fn(*mut i64) -> u8>(b"answer")
                .expect("typed answer export should exist");
            let greet = library
                .get::<unsafe extern "system" fn(ArcanaStrView, *mut ArcanaOwnedStr) -> u8>(
                    b"greet",
                )
                .expect("typed greet export should exist");
            let prefix = library
                .get::<unsafe extern "system" fn(*mut ArcanaOwnedBytes) -> u8>(b"prefix")
                .expect("typed prefix export should exist");
            let byte_len = library
                .get::<unsafe extern "system" fn(ArcanaBytesView, *mut i64) -> u8>(b"byte_len")
                .expect("typed byte_len export should exist");
            let echo_pair = library
                .get::<unsafe extern "system" fn(
                    ArcanaPairView__Pair__Str__Int,
                    *mut ArcanaPairOwned__Pair__Str__Int,
                ) -> u8>(b"echo_pair")
                .expect("typed pair export should exist");
            let last_error = library
                .get::<unsafe extern "system" fn(*mut usize) -> *mut u8>(b"arcana_last_error_alloc")
                .expect("last-error export should exist");
            let free_bytes = library
                .get::<unsafe extern "system" fn(*mut u8, usize)>(b"arcana_bytes_free")
                .expect("free export should exist");
            let mut result = 0i64;
            let ok = answer(&mut result);
            if ok == 0 {
                let err =
                    read_allocated_utf8(&last_error, &free_bytes).expect("last error should read");
                panic!("typed dll export failed: {err}");
            }
            assert_eq!(result, 11);

            let mut greeting = ArcanaOwnedStr::default();
            let name = b"arcana";
            let ok = greet(
                ArcanaStrView {
                    ptr: name.as_ptr(),
                    len: name.len(),
                },
                &mut greeting,
            );
            if ok == 0 {
                let err =
                    read_allocated_utf8(&last_error, &free_bytes).expect("last error should read");
                panic!("typed greet export failed: {err}");
            }
            let greeting_text = read_owned_utf8(greeting, &free_bytes).expect("greeting utf8");
            assert_eq!(greeting_text, "hello arcana");

            let mut prefix_bytes = ArcanaOwnedBytes::default();
            let ok = prefix(&mut prefix_bytes);
            if ok == 0 {
                let err =
                    read_allocated_utf8(&last_error, &free_bytes).expect("last error should read");
                panic!("typed prefix export failed: {err}");
            }
            let prefix_text = String::from_utf8(
                read_owned_bytes(prefix_bytes, &free_bytes).expect("prefix bytes should read"),
            )
            .expect("prefix bytes should decode");
            assert_eq!(prefix_text, "arc");

            let payload = b"bundle";
            let mut len_result = 0i64;
            let ok = byte_len(
                ArcanaBytesView {
                    ptr: payload.as_ptr(),
                    len: payload.len(),
                },
                &mut len_result,
            );
            if ok == 0 {
                let err =
                    read_allocated_utf8(&last_error, &free_bytes).expect("last error should read");
                panic!("typed byte_len export failed: {err}");
            }
            assert_eq!(len_result, 6);

            let pair_label = b"pair";
            let mut echoed_pair = ArcanaPairOwned__Pair__Str__Int::default();
            let ok = echo_pair(
                ArcanaPairView__Pair__Str__Int {
                    left: ArcanaStrView {
                        ptr: pair_label.as_ptr(),
                        len: pair_label.len(),
                    },
                    right: 17,
                },
                &mut echoed_pair,
            );
            if ok == 0 {
                let err =
                    read_allocated_utf8(&last_error, &free_bytes).expect("last error should read");
                panic!("typed pair export failed: {err}");
            }
            let echoed_left =
                read_owned_utf8(echoed_pair.left, &free_bytes).expect("pair text should read");
            assert_eq!(echoed_left, "pair");
            assert_eq!(echoed_pair.right, 17);
        }

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    unsafe fn read_allocated_utf8(
        alloc: &libloading::Symbol<unsafe extern "system" fn(*mut usize) -> *mut u8>,
        free: &libloading::Symbol<unsafe extern "system" fn(*mut u8, usize)>,
    ) -> Result<String, String> {
        let mut len = 0usize;
        let ptr = unsafe { alloc(&mut len) };
        if ptr.is_null() {
            return Err("allocation returned null".to_string());
        }
        let bytes = unsafe { std::slice::from_raw_parts(ptr, len) }.to_vec();
        unsafe { free(ptr, len) };
        String::from_utf8(bytes).map_err(|e| format!("utf8 decode failed: {e}"))
    }

    #[cfg(windows)]
    unsafe fn read_owned_bytes(
        owned: ArcanaOwnedBytes,
        free: &libloading::Symbol<unsafe extern "system" fn(*mut u8, usize)>,
    ) -> Result<Vec<u8>, String> {
        if owned.ptr.is_null() {
            return Ok(Vec::new());
        }
        let bytes = unsafe { std::slice::from_raw_parts(owned.ptr, owned.len) }.to_vec();
        unsafe { free(owned.ptr, owned.len) };
        Ok(bytes)
    }

    #[cfg(windows)]
    unsafe fn read_owned_utf8(
        owned: ArcanaOwnedStr,
        free: &libloading::Symbol<unsafe extern "system" fn(*mut u8, usize)>,
    ) -> Result<String, String> {
        String::from_utf8(unsafe {
            read_owned_bytes(
                ArcanaOwnedBytes {
                    ptr: owned.ptr,
                    len: owned.len,
                },
                free,
            )
        }?)
        .map_err(|e| format!("utf8 decode failed: {e}"))
    }
}
