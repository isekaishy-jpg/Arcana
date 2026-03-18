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

    fn write_test_wav_with_format(path: &Path, sample_rate_hz: u32, channels: u16) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("parent directories should be created");
        }
        let bits_per_sample = 16u16;
        let frame_count = 64u32;
        let block_align = channels * (bits_per_sample / 8);
        let byte_rate = sample_rate_hz * u32::from(block_align);
        let data_len = frame_count * u32::from(block_align);
        let mut bytes = Vec::with_capacity(44 + data_len as usize);
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
        bytes.extend_from_slice(b"WAVE");
        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&channels.to_le_bytes());
        bytes.extend_from_slice(&sample_rate_hz.to_le_bytes());
        bytes.extend_from_slice(&byte_rate.to_le_bytes());
        bytes.extend_from_slice(&block_align.to_le_bytes());
        bytes.extend_from_slice(&bits_per_sample.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_len.to_le_bytes());
        for frame in 0..frame_count {
            let sample = if frame % 8 < 4 {
                i16::MAX / 8
            } else {
                -(i16::MAX / 8)
            };
            bytes.extend_from_slice(&sample.to_le_bytes());
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        fs::write(path, bytes).expect("wav fixture should write");
    }

    fn write_test_wav(path: &Path) {
        write_test_wav_with_format(path, 48_000, 2);
    }

    fn write_test_bmp(path: &Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("parent directories should be created");
        }
        let width = 2u32;
        let height = 2u32;
        let row_stride = 8u32;
        let pixel_data_len = row_stride * height;
        let file_len = 54u32 + pixel_data_len;
        let mut bytes = Vec::with_capacity(file_len as usize);
        bytes.extend_from_slice(b"BM");
        bytes.extend_from_slice(&file_len.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(&54u32.to_le_bytes());
        bytes.extend_from_slice(&40u32.to_le_bytes());
        bytes.extend_from_slice(&(width as i32).to_le_bytes());
        bytes.extend_from_slice(&(height as i32).to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&24u16.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&pixel_data_len.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&[0x00, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0x00, 0x00]);
        bytes.extend_from_slice(&[0xFF, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0x00, 0x00]);
        fs::write(path, bytes).expect("bmp fixture should write");
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

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("repo root should exist")
            .to_path_buf()
    }

    fn add_std_dep(dir: &Path) {
        let std_path = repo_root()
            .join("std")
            .display()
            .to_string()
            .replace('\\', "/");
        fs::write(
            dir.join("book.toml"),
            format!(
                "name = \"app\"\nkind = \"app\"\n\n[deps]\nstd = {{ path = \"{std_path}\" }}\n"
            ),
        )
        .expect("book manifest should write");
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
        write_app_workspace(
            &dir,
            concat!(
                "fn touch():\n",
                "    return\n",
                "fn helper(value: Int) -> Int:\n",
                "    touch :: :: call\n",
                "    let mut i = 0\n",
                "    let mut bumped = value\n",
                "    while i < 1:\n",
                "        bumped += 1\n",
                "        i += 1\n",
                "    return bumped\n",
                "fn main() -> Int:\n",
                "    let base = 8\n",
                "    if base >= 8:\n",
                "        return helper :: value = base :: call\n",
                "    else:\n",
                "        return 0\n",
            ),
        );

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
    fn package_workspace_runs_unit_returning_windows_exe_bundle() {
        let dir = temp_dir("windows_exe_unit_main");
        write_app_workspace(
            &dir,
            concat!(
                "fn touch():\n",
                "    return\n",
                "fn main():\n",
                "    touch :: :: call\n",
                "    return\n",
            ),
        );

        let bundle = package_workspace(dir.clone(), BuildTarget::windows_exe(), None, None)
            .expect("package should succeed");
        let exe_path = bundle.bundle_dir.join(&bundle.root_artifact);
        let status = Command::new(&exe_path)
            .status()
            .expect("staged bundle should launch");
        assert_eq!(status.code(), Some(0));

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn package_workspace_stages_windows_exe_bundle_with_owner_activation() {
        let dir = temp_dir("windows_exe_owner");
        write_app_workspace(
            &dir,
            concat!(
                "obj Counter:\n",
                "    value: Int\n",
                "\n",
                "create Session [Counter] scope-exit:\n",
                "    done: when Counter.value >= 10 hold [Counter]\n",
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

        let bundle = package_workspace(dir.clone(), BuildTarget::windows_exe(), None, None)
            .expect("package should succeed");
        let exe_path = bundle.bundle_dir.join(&bundle.root_artifact);
        let status = Command::new(&exe_path)
            .status()
            .expect("staged owner bundle should launch");
        assert_eq!(status.code(), Some(10));

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn package_workspace_stages_windows_exe_bundle_with_owner_context_hooks() {
        let dir = temp_dir("windows_exe_owner_context");
        write_app_workspace(
            &dir,
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
                "    done: when Counter.value == 3 hold [Counter]\n",
                "\n",
                "Session\n",
                "Counter\n",
                "fn main() -> Int:\n",
                "    let start = SessionCtx :: base = 1 :: call\n",
                "    Session :: start :: call\n",
                "    Counter.value\n",
                "    Counter.value = 3\n",
                "    let resume_ctx = SessionCtx :: base = 2 :: call\n",
                "    let resumed = Session :: resume_ctx :: call\n",
                "    return resumed.Counter.value\n",
            ),
        );

        let bundle = package_workspace(dir.clone(), BuildTarget::windows_exe(), None, None)
            .expect("package should succeed");
        let exe_path = bundle.bundle_dir.join(&bundle.root_artifact);
        let status = Command::new(&exe_path)
            .status()
            .expect("staged owner context bundle should launch");
        assert_eq!(status.code(), Some(5));

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
                "fn touch():\n",
                "    return\n",
                "fn answer_impl(value: Int) -> Int:\n",
                "    touch :: :: call\n",
                "    let mut i = 0\n",
                "    let mut doubled = 0\n",
                "    while i < 2:\n",
                "        doubled += value\n",
                "        i += 1\n",
                "    return doubled\n",
                "export fn answer() -> Int:\n",
                "    let base = 6\n",
                "    if base > 4:\n",
                "        return answer_impl :: value = base :: call\n",
                "    else:\n",
                "        return 0\n",
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
            assert_eq!(result, 12);

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
    #[test]
    fn package_workspace_stages_loadable_windows_dll_bundle_with_owner_activation() {
        let dir = temp_dir("windows_dll_owner");
        write_lib_workspace(
            &dir,
            concat!(
                "obj Counter:\n",
                "    value: Int\n",
                "\n",
                "create Session [Counter] scope-exit:\n",
                "    done: when Counter.value >= 4 hold [Counter]\n",
                "\n",
                "Session\n",
                "Counter\n",
                "export fn answer() -> Int:\n",
                "    let active = Session :: :: call\n",
                "    Counter.value = 2\n",
                "    Counter.value += 2\n",
                "    let resumed = Session :: :: call\n",
                "    return resumed.Counter.value\n",
            ),
        );

        let bundle = package_workspace(dir.clone(), BuildTarget::windows_dll(), None, None)
            .expect("dll package should succeed");
        let dll_path = bundle.bundle_dir.join(&bundle.root_artifact);

        unsafe {
            let library = Library::new(&dll_path).expect("dll should load");
            let answer = library
                .get::<unsafe extern "system" fn(*mut i64) -> u8>(b"answer")
                .expect("typed answer export should exist");
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
            assert_eq!(result, 4);
        }

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn package_workspace_runs_native_window_owner_app_bundle() {
        let dir = temp_dir("windows_window_owner");
        write_app_workspace(
            &dir,
            concat!(
                "import std.io\n",
                "import std.window\n",
                "use std.result.Result\n",
                "obj WindowState:\n",
                "    win: std.window.Window\n",
                "\n",
                "create Session [WindowState] scope-exit:\n",
                "    closed: when not (std.window.alive :: WindowState.win :: call) hold [WindowState]\n",
                "\n",
                "Session\n",
                "WindowState\n",
                "fn run_with_window(take win: std.window.Window) -> Int:\n",
                "    let active = Session :: :: call\n",
                "    WindowState.win = win\n",
                "    let resumed = Session :: :: call\n",
                "    let size = std.window.size :: resumed.WindowState.win :: call\n",
                "    std.io.print[Int] :: size.0 :: call\n",
                "    std.io.print[Int] :: size.1 :: call\n",
                "    let close = std.window.close :: resumed.WindowState.win :: call\n",
                "    if close :: :: is_err:\n",
                "        return 3\n",
                "    return 0\n",
                "\n",
                "fn run() -> Int:\n",
                "    return match (std.window.open :: \"Arcana Owner\", 160, 120 :: call):\n",
                "        Result.Ok(win) => run_with_window :: win :: call\n",
                "        Result.Err(_) => 1\n",
                "fn main() -> Int:\n",
                "    return run :: :: call\n",
            ),
        );
        add_std_dep(&dir);

        let bundle = package_workspace(
            dir.clone(),
            BuildTarget::windows_exe(),
            Some("app".to_string()),
            None,
        )
        .expect("window owner package should succeed");
        let exe_path = bundle.bundle_dir.join(&bundle.root_artifact);
        let output = Command::new(&exe_path)
            .current_dir(&bundle.bundle_dir)
            .output()
            .expect("staged owner window bundle should launch");
        assert_eq!(output.status.code(), Some(0));
        assert_eq!(
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .collect::<Vec<_>>(),
            vec!["160120"]
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn package_workspace_runs_native_window_canvas_app_bundle() {
        let dir = temp_dir("windows_window_canvas");
        write_test_bmp(&dir.join("fixture").join("sprite.bmp"));
        write_app_workspace(
            &dir,
            concat!(
                "import std.canvas\n",
                "import std.io\n",
                "import std.window\n",
                "use std.result.Result\n",
                "fn draw_image(edit win: std.window.Window, read img: std.canvas.Image) -> Int:\n",
                "    let img_size = std.canvas.image_size :: img :: call\n",
                "    if img_size.0 != 2:\n",
                "        return 4\n",
                "    if img_size.1 != 2:\n",
                "        return 5\n",
                "    std.canvas.blit :: win, img, 8 :: call\n",
                "        y = 9\n",
                "    return 0\n",
                "fn run(take win: std.window.Window) -> Int:\n",
                "    let mut win = win\n",
                "    if not (std.window.alive :: win :: call):\n",
                "        return 2\n",
                "    let size = std.window.size :: win :: call\n",
                "    std.io.print[Int] :: size.0 :: call\n",
                "    std.io.print[Int] :: size.1 :: call\n",
                "    let color = std.canvas.rgb :: 10, 20, 30 :: call\n",
                "    std.canvas.fill :: win, color :: call\n",
                "    let image_status = match (std.canvas.image_load :: \"sprite.bmp\" :: call):\n",
                "        Result.Ok(img) => draw_image :: win, img :: call\n",
                "        Result.Err(_) => 6\n",
                "    if image_status != 0:\n",
                "        return image_status\n",
                "    std.canvas.present :: win :: call\n",
                "    let close = std.window.close :: win :: call\n",
                "    if close :: :: is_err:\n",
                "        return 3\n",
                "    return 0\n",
                "fn main() -> Int:\n",
                "    return match (std.window.open :: \"Arcana\", 320, 200 :: call):\n",
                "        Result.Ok(win) => run :: win :: call\n",
                "        Result.Err(_) => 1\n",
            ),
        );
        add_std_dep(&dir);

        let bundle = package_workspace(
            dir.clone(),
            BuildTarget::windows_exe(),
            Some("app".to_string()),
            None,
        )
        .expect("window canvas package should succeed");
        let exe_path = bundle.bundle_dir.join(&bundle.root_artifact);
        fs::copy(
            dir.join("fixture").join("sprite.bmp"),
            bundle.bundle_dir.join("sprite.bmp"),
        )
        .expect("sprite fixture should copy into bundle");
        let output = Command::new(&exe_path)
            .current_dir(&bundle.bundle_dir)
            .output()
            .expect("staged bundle should launch");
        assert_eq!(output.status.code(), Some(0));
        assert_eq!(
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .collect::<Vec<_>>(),
            vec!["320200"]
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn package_workspace_runs_native_audio_app_bundle() {
        let dir = temp_dir("windows_audio");
        write_test_wav(&dir.join("fixture").join("clip.wav"));
        write_app_workspace(
            &dir,
            concat!(
                "import std.audio\n",
                "import std.io\n",
                "use std.result.Result\n",
                "fn use_playback(take device: std.audio.AudioDevice, take playback: std.audio.AudioPlayback) -> Int:\n",
                "    let stop = playback :: :: stop\n",
                "    if stop :: :: is_err:\n",
                "        return 4\n",
                "    let close = std.audio.output_close :: device :: call\n",
                "    if close :: :: is_err:\n",
                "        return 5\n",
                "    return 0\n",
                "fn use_clip(take device: std.audio.AudioDevice, read clip: std.audio.AudioBuffer) -> Int:\n",
                "    let mut device = device\n",
                "    std.io.print[Int] :: (std.audio.buffer_sample_rate_hz :: clip :: call) :: call\n",
                "    let playback_result = std.audio.play_buffer :: device, clip :: call\n",
                "    return match playback_result:\n",
                "        Result.Ok(value) => use_playback :: device, value :: call\n",
                "        Result.Err(_) => 3\n",
                "fn main() -> Int:\n",
                "    return match (std.audio.default_output :: :: call):\n",
                "        Result.Ok(device) => match (std.audio.buffer_load_wav :: \"clip.wav\" :: call):\n",
                "            Result.Ok(clip) => use_clip :: device, clip :: call\n",
                "            Result.Err(_) => 2\n",
                "        Result.Err(_) => 1\n",
            ),
        );
        add_std_dep(&dir);

        let bundle = package_workspace(
            dir.clone(),
            BuildTarget::windows_exe(),
            Some("app".to_string()),
            None,
        )
        .expect("audio package should succeed");
        let exe_path = bundle.bundle_dir.join(&bundle.root_artifact);
        fs::copy(
            dir.join("fixture").join("clip.wav"),
            bundle.bundle_dir.join("clip.wav"),
        )
        .expect("clip fixture should copy into bundle");
        let output = Command::new(&exe_path)
            .current_dir(&bundle.bundle_dir)
            .output()
            .expect("staged audio bundle should launch");
        assert_eq!(output.status.code(), Some(0));
        assert_eq!(
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .collect::<Vec<_>>(),
            vec!["48000"]
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn package_workspace_rejects_native_audio_buffer_format_mismatch() {
        let dir = temp_dir("windows_audio_mismatch");
        write_test_wav_with_format(&dir.join("fixture").join("clip_48k_stereo.wav"), 48_000, 2);
        write_test_wav_with_format(&dir.join("fixture").join("clip_44k_stereo.wav"), 44_100, 2);
        write_app_workspace(
            &dir,
            concat!(
                "import std.audio\n",
                "use std.result.Result\n",
                "fn mismatch_path(read device: std.audio.AudioDevice) -> Str:\n",
                "    if (std.audio.output_sample_rate_hz :: device :: call) == 48000:\n",
                "        return \"clip_44k_stereo.wav\"\n",
                "    return \"clip_48k_stereo.wav\"\n",
                "fn use_device(take device: std.audio.AudioDevice) -> Int:\n",
                "    let mut device = device\n",
                "    let path = mismatch_path :: device :: call\n",
                "    return match (std.audio.buffer_load_wav :: path :: call):\n",
                "        Result.Ok(clip) => match (std.audio.play_buffer :: device, clip :: call):\n",
                "            Result.Ok(_) => 4\n",
                "            Result.Err(_) => 0\n",
                "        Result.Err(_) => 3\n",
                "fn main() -> Int:\n",
                "    return match (std.audio.default_output :: :: call):\n",
                "        Result.Ok(device) => use_device :: device :: call\n",
                "        Result.Err(_) => 1\n",
            ),
        );
        add_std_dep(&dir);

        let bundle = package_workspace(
            dir.clone(),
            BuildTarget::windows_exe(),
            Some("app".to_string()),
            None,
        )
        .expect("audio mismatch package should succeed");
        let exe_path = bundle.bundle_dir.join(&bundle.root_artifact);
        fs::copy(
            dir.join("fixture").join("clip_48k_stereo.wav"),
            bundle.bundle_dir.join("clip_48k_stereo.wav"),
        )
        .expect("stereo clip fixture should copy into bundle");
        fs::copy(
            dir.join("fixture").join("clip_44k_stereo.wav"),
            bundle.bundle_dir.join("clip_44k_stereo.wav"),
        )
        .expect("44k clip fixture should copy into bundle");
        let output = Command::new(&exe_path)
            .current_dir(&bundle.bundle_dir)
            .output()
            .expect("staged audio mismatch bundle should launch");
        assert_eq!(output.status.code(), Some(0));

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
