use std::path::PathBuf;

use arcana_frontend::check_workspace_graph;
use arcana_package::{
    BuildOutputKey, BuildTarget, DistributionBundle, GrimoireKind, WorkspaceGraph,
    default_distribution_dir_for_build, execute_build_with_context_and_progress,
    load_workspace_graph, plan_package_build_for_target_with_context, plan_workspace,
    prepare_build_from_workspace, read_lockfile, stage_distribution_bundle_for_build,
    write_lockfile,
};

use crate::build_context::{build_execution_context_for_target, render_build_progress};

#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn package_workspace(
    workspace_dir: PathBuf,
    target: BuildTarget,
    member: Option<String>,
    out_dir: Option<PathBuf>,
) -> Result<DistributionBundle, String> {
    package_workspace_with_product(workspace_dir, target, None, member, out_dir)
}

pub(crate) fn package_workspace_with_product(
    workspace_dir: PathBuf,
    target: BuildTarget,
    product: Option<String>,
    member: Option<String>,
    out_dir: Option<PathBuf>,
) -> Result<DistributionBundle, String> {
    #[cfg(test)]
    let _test_guard = crate::heavy_test_mutex()
        .lock()
        .expect("heavy cli test mutex should not be poisoned");
    let graph = load_workspace_graph(&workspace_dir)?;
    let packaged_member = resolve_package_member(&graph, member.as_deref())?;
    target.artifact_file_name(&packaged_member.kind)?;
    let packaged_member_id = packaged_member.package_id.clone();
    let packaged_member_name = packaged_member.name.clone();

    let order = plan_workspace(&graph)?;
    let checked = check_workspace_graph(&graph)?;
    let (workspace, resolved_workspace) = checked.into_workspace_parts();
    let prepared = prepare_build_from_workspace(&graph, workspace, resolved_workspace)?;
    let lock_path = graph.root_dir.join("Arcana.lock");
    let existing_lock = read_lockfile(&lock_path)?;
    let execution_context = build_execution_context_for_target(&target, product.clone())?;
    let statuses = plan_package_build_for_target_with_context(
        &graph,
        &order,
        &prepared,
        existing_lock.as_ref(),
        target.clone(),
        &packaged_member_id,
        &execution_context,
    )?;
    let build_key = statuses
        .iter()
        .find(|status| status.member() == packaged_member_id && status.target() == &target)
        .map(|status| status.build_key().clone())
        .unwrap_or_else(|| BuildOutputKey::new(target.clone(), product.clone()));
    let output_dir = out_dir.unwrap_or_else(|| {
        default_distribution_dir_for_build(&graph, &packaged_member_id, &build_key)
    });
    execute_build_with_context_and_progress(
        &graph,
        &prepared,
        &statuses,
        &execution_context,
        |progress| println!("{}", render_build_progress(progress)),
    )?;
    write_lockfile(&graph, &order, &statuses)?;
    stage_distribution_bundle_for_build(
        &graph,
        &statuses,
        &packaged_member_id,
        &build_key,
        &output_dir,
    )
    .map(|mut bundle| {
        bundle.member = packaged_member_name;
        bundle
    })
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
    #![allow(dead_code, unused_imports)]

    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
    use std::ffi::{CStr, c_char, c_void};
    use std::fs;
    use std::io::Read;
    use std::ops::{Deref, DerefMut};
    use std::path::Path;
    use std::thread;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
    use libloading::Library;
    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
    use std::process::{Child, Command, Stdio};
    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
    type HWND = *mut c_void;

    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
    const WM_CLOSE: u32 = 0x0010;
    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
    const WM_MOUSEMOVE: u32 = 0x0200;
    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
    const WM_LBUTTONDOWN: u32 = 0x0201;
    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
    const WM_LBUTTONUP: u32 = 0x0202;
    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
    const WM_CHAR: u32 = 0x0102;
    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
    const WM_IME_STARTCOMPOSITION: u32 = 0x010D;
    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
    const WM_IME_ENDCOMPOSITION: u32 = 0x010E;

    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
    #[link(name = "user32")]
    unsafe extern "system" {
        fn EnumWindows(
            callback: Option<unsafe extern "system" fn(HWND, isize) -> i32>,
            lparam: isize,
        ) -> i32;
        fn GetWindowTextLengthW(hwnd: HWND) -> i32;
        fn GetWindowTextW(hwnd: HWND, text: *mut u16, max_count: i32) -> i32;
        fn GetWindowThreadProcessId(hwnd: HWND, pid: *mut u32) -> u32;
        fn IsWindowVisible(hwnd: HWND) -> i32;
        fn SendMessageW(hwnd: HWND, msg: u32, wparam: usize, lparam: isize) -> isize;
        fn SetForegroundWindow(hwnd: HWND) -> i32;
    }

    use super::*;

    fn temp_dir(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let dir = repo_root()
            .join("target")
            .join("arcana-cli-package-tests")
            .join(format!("{label}_{unique}"));
        fs::create_dir_all(&dir).expect("temp dir should be created");
        dir
    }

    fn repo_temp_dir(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let dir = repo_root()
            .join("target")
            .join(format!("arcana-cli-package-workspace-{label}_{unique}"));
        fs::create_dir_all(&dir).expect("repo temp dir should be created");
        dir
    }

    fn write_file(path: &Path, text: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("parent directories should be created");
        }
        fs::write(path, text).expect("file should write");
    }

    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
    struct WindowSearch {
        pid: u32,
        title_contains: String,
        hwnd: HWND,
        title: String,
    }

    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
    struct WindowListSearch {
        pid: u32,
        windows: Vec<(HWND, String)>,
    }

    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
    struct TestChild {
        child: Child,
    }

    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
    impl TestChild {
        fn new(child: Child) -> Self {
            Self { child }
        }
    }

    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
    impl Deref for TestChild {
        type Target = Child;

        fn deref(&self) -> &Self::Target {
            &self.child
        }
    }

    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
    impl DerefMut for TestChild {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.child
        }
    }

    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
    impl Drop for TestChild {
        fn drop(&mut self) {
            if matches!(self.child.try_wait(), Ok(None)) {
                let _ = self.child.kill();
                let _ = self.child.wait();
            }
        }
    }

    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
    unsafe extern "system" fn collect_process_window(hwnd: HWND, lparam: isize) -> i32 {
        let search = unsafe { &mut *(lparam as *mut WindowSearch) };
        let mut pid = 0u32;
        unsafe {
            GetWindowThreadProcessId(hwnd, &mut pid);
        }
        if pid != search.pid {
            return 1;
        }
        if unsafe { IsWindowVisible(hwnd) } == 0 {
            return 1;
        }
        let title_len = unsafe { GetWindowTextLengthW(hwnd) };
        let mut title = String::new();
        if title_len > 0 {
            let mut buffer = vec![0u16; usize::try_from(title_len).unwrap_or(0) + 1];
            let read = unsafe { GetWindowTextW(hwnd, buffer.as_mut_ptr(), title_len + 1) };
            if read > 0 {
                title = String::from_utf16_lossy(&buffer[..usize::try_from(read).unwrap_or(0)]);
            }
        }
        if !search.title_contains.is_empty() && !title.contains(&search.title_contains) {
            return 1;
        }
        search.hwnd = hwnd;
        search.title = title;
        0
    }

    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
    unsafe extern "system" fn collect_process_windows(hwnd: HWND, lparam: isize) -> i32 {
        let search = unsafe { &mut *(lparam as *mut WindowListSearch) };
        let mut pid = 0u32;
        unsafe {
            GetWindowThreadProcessId(hwnd, &mut pid);
        }
        if pid != search.pid {
            return 1;
        }
        if unsafe { IsWindowVisible(hwnd) } == 0 {
            return 1;
        }
        let title_len = unsafe { GetWindowTextLengthW(hwnd) };
        let mut title = String::new();
        if title_len > 0 {
            let mut buffer = vec![0u16; usize::try_from(title_len).unwrap_or(0) + 1];
            let read = unsafe { GetWindowTextW(hwnd, buffer.as_mut_ptr(), title_len + 1) };
            if read > 0 {
                title = String::from_utf16_lossy(&buffer[..usize::try_from(read).unwrap_or(0)]);
            }
        }
        search.windows.push((hwnd, title));
        1
    }

    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
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

    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
    fn compile_c_header_smoke(bundle_dir: &Path, header_name: &str) {
        let source_path = bundle_dir.join("consumer.c");
        let object_path = bundle_dir.join("consumer.obj");
        let header_include = format!(
            concat!(
                "#include <stddef.h>\n",
                "#include <stdint.h>\n",
                "#include \"{header_name}\"\n",
                "\n",
                "static uint8_t smoke(void) {{\n",
                "    int64_t answer_result = 0;\n",
                "    int64_t len_result = 0;\n",
                "    const uint8_t label_bytes[] = {{ 'a', 'r', 'c', 'a', 'n', 'a' }};\n",
                "    ArcanaViewV1 name = {{ label_bytes, 6, 1, 1, 1, 1 }};\n",
                "    ArcanaOwnedStr out_text = {{ 0 }};\n",
                "    ArcanaOwnedBytes out_bytes = {{ 0 }};\n",
                "    ArcanaPairView__Pair__Str__Int pair = {{ name, 7 }};\n",
                "    answer(&answer_result);\n",
                "    greet(name, &out_text);\n",
                "    prefix(&out_bytes);\n",
                "    byte_len((ArcanaViewV1){{ label_bytes, 6, 1, 1, 1, 0 }}, &len_result);\n",
                "    echo_pair(pair, (ArcanaPairOwned__Pair__Str__Int*)0);\n",
                "    return 0;\n",
                "}}\n"
            ),
            header_name = header_name
        );
        write_file(&source_path, &header_include);

        let mut attempts = Vec::new();

        let cl_result = Command::new("cl")
            .arg("/nologo")
            .arg("/c")
            .arg("/TC")
            .arg(&source_path)
            .arg(format!("/I{}", bundle_dir.display()))
            .arg(format!("/Fo{}", object_path.display()))
            .current_dir(bundle_dir)
            .output();
        match cl_result {
            Ok(output) if output.status.success() => return,
            Ok(output) => attempts.push(format!(
                "cl from PATH failed:\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            )),
            Err(err) if err.kind() != std::io::ErrorKind::NotFound => {
                attempts.push(format!("cl from PATH failed to launch: {err}"));
            }
            Err(_) => {}
        }

        if let Some(result) =
            try_compile_c_header_with_vcvars(&source_path, bundle_dir, &object_path)
        {
            match result {
                Ok(()) => return,
                Err(err) => attempts.push(err),
            }
        }

        for compiler in ["clang", "gcc"] {
            match Command::new(compiler)
                .arg("-std=c11")
                .arg("-c")
                .arg(&source_path)
                .arg("-I")
                .arg(bundle_dir)
                .arg("-o")
                .arg(&object_path)
                .current_dir(bundle_dir)
                .output()
            {
                Ok(output) if output.status.success() => return,
                Ok(output) => attempts.push(format!(
                    "{compiler} failed:\nstdout:\n{}\nstderr:\n{}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                )),
                Err(err) if err.kind() != std::io::ErrorKind::NotFound => {
                    attempts.push(format!("{compiler} failed to launch: {err}"));
                }
                Err(_) => {}
            }
        }

        if !attempts.is_empty() {
            panic!(
                "C header smoke failed for `{}`:\n\n{}",
                source_path.display(),
                attempts.join("\n\n")
            );
        }
        eprintln!(
            "skipping C header smoke for `{}`: no usable C compiler found on PATH",
            source_path.display()
        );
    }

    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
    fn try_compile_c_header_with_vcvars(
        source_path: &Path,
        include_dir: &Path,
        object_path: &Path,
    ) -> Option<Result<(), String>> {
        let vcvars_path = find_msvc_vcvars64()?;
        let script_path = include_dir.join("compile_consumer_msvc.bat");
        let script = format!(
            concat!(
                "@echo off\n",
                "call \"{}\" >nul\n",
                "if errorlevel 1 exit /b %errorlevel%\n",
                "cl /nologo /c /TC \"{}\" /I\"{}\" /Fo\"{}\"\n"
            ),
            cmd_compatible_path(&vcvars_path),
            cmd_compatible_path(source_path),
            cmd_compatible_path(include_dir),
            cmd_compatible_path(object_path)
        );
        write_file(&script_path, &script);
        let script_cmd = cmd_compatible_path(&script_path);
        Some(
            match Command::new("cmd")
                .args(["/d", "/s", "/c", script_cmd.as_str()])
                .current_dir(include_dir)
                .output()
            {
                Ok(output) if output.status.success() => Ok(()),
                Ok(output) => Err(format!(
                    "MSVC vcvars64 compile failed:\nstdout:\n{}\nstderr:\n{}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                )),
                Err(err) => Err(format!("MSVC vcvars64 compile failed to launch: {err}")),
            },
        )
    }

    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
    fn find_msvc_vcvars64() -> Option<PathBuf> {
        let program_files = [
            PathBuf::from(r"C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe"),
            PathBuf::from(r"C:\Program Files\Microsoft Visual Studio\Installer\vswhere.exe"),
        ];
        for vswhere in program_files {
            if !vswhere.is_file() {
                continue;
            }
            let Ok(output) = Command::new(&vswhere)
                .args([
                    "-latest",
                    "-products",
                    "*",
                    "-requires",
                    "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
                    "-property",
                    "installationPath",
                ])
                .output()
            else {
                continue;
            };
            if !output.status.success() {
                continue;
            }
            let installation = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if installation.is_empty() {
                continue;
            }
            let vcvars = PathBuf::from(installation).join(r"VC\Auxiliary\Build\vcvars64.bat");
            if vcvars.is_file() {
                return Some(vcvars);
            }
        }
        None
    }

    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
    fn cmd_compatible_path(path: &Path) -> String {
        let rendered = path.display().to_string();
        if let Some(stripped) = rendered.strip_prefix(r"\\?\") {
            stripped.to_string()
        } else {
            rendered
        }
    }

    fn write_std_text_grimoire(dir: &Path) {
        write_file(
            &dir.join("std/book.toml"),
            "name = \"std\"\nkind = \"lib\"\n",
        );
        write_file(
            &dir.join("std/src/book.arc"),
            "import text\nimport kernel.text\n",
        );
        write_file(&dir.join("std/src/types.arc"), "// std types\n");
        write_file(
            &dir.join("std/src/text.arc"),
            concat!(
                "import std.kernel.text\n",
                "export fn from_str_utf8(text: Str) -> Bytes:\n",
                "    return std.kernel.text.bytes_from_str_utf8 :: text :: call\n",
                "export fn len(read bytes: Bytes) -> Int:\n",
                "    return std.kernel.text.bytes_len :: bytes :: call\n",
            ),
        );
        write_file(
            &dir.join("std/src/kernel/text.arc"),
            concat!(
                "intrinsic fn bytes_from_str_utf8(text: Str) -> Bytes = HostBytesFromStrUtf8\n",
                "intrinsic fn bytes_len(read bytes: Bytes) -> Int = HostBytesLen\n",
            ),
        );
    }

    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct ArcanaViewV1 {
        ptr: *const u8,
        len: usize,
        stride_bytes: usize,
        family: u32,
        element_size: u32,
        flags: u32,
    }

    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct ArcanaOwnedStr {
        ptr: *mut u8,
        len: usize,
    }

    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct ArcanaOwnedBytes {
        ptr: *mut u8,
        len: usize,
    }

    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct ArcanaPairView__Pair__Str__Int {
        left: ArcanaViewV1,
        right: i64,
    }

    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct ArcanaPairOwned__Pair__Str__Int {
        left: ArcanaOwnedStr,
        right: i64,
    }

    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
    #[repr(C)]
    struct TestArcanaCabiProductApiV1 {
        descriptor_size: usize,
        package_name: *const c_char,
        product_name: *const c_char,
        role: *const c_char,
        contract_id: *const c_char,
        contract_version: u32,
        role_ops: *const c_void,
        reserved0: *const c_void,
        reserved1: *const c_void,
    }

    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
    unsafe fn read_cabi_utf8_field(ptr: *const c_char) -> String {
        unsafe { CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned()
    }

    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
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
        assert!(
            bundle.bundle_dir.starts_with(repo_root().join("dist")),
            "expected packaged bundle under repo dist, got {}",
            bundle.bundle_dir.display()
        );
        let exe_path = bundle.bundle_dir.join(&bundle.root_artifact);
        let status = Command::new(&exe_path)
            .arg("alpha")
            .status()
            .expect("staged bundle should launch");
        assert_eq!(status.code(), Some(9));
        assert!(bundle.support_files.is_empty());
        assert!(
            !bundle
                .bundle_dir
                .join("app.exe.arcana-bundle.toml")
                .exists(),
            "did not expect staged exe native manifest"
        );
        assert!(
            !bundle.bundle_dir.join("arcana.bundle.toml").exists(),
            "did not expect staged distribution manifest"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
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

    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
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

        let bundle = package_workspace(dir.clone(), BuildTarget::windows_exe(), None, None)
            .expect("package should succeed");
        let exe_path = bundle.bundle_dir.join(&bundle.root_artifact);
        let status = Command::new(&exe_path)
            .status()
            .expect("staged owner bundle should launch");
        assert_eq!(status.code(), Some(10));

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
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
                "    done: when Counter.value == 3 retain [Counter]\n",
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

    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
    #[test]
    fn package_workspace_stages_windows_exe_bundle_with_active_owner_reentry_context() {
        let dir = temp_dir("windows_exe_owner_reentry_context");
        write_app_workspace(
            &dir,
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
        add_std_dep(&dir);

        let bundle = package_workspace(dir.clone(), BuildTarget::windows_exe(), None, None)
            .expect("package should succeed");
        let exe_path = bundle.bundle_dir.join(&bundle.root_artifact);
        let status = Command::new(&exe_path)
            .status()
            .expect("staged reentry-context owner bundle should launch");
        assert_eq!(status.code(), Some(2));

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
    #[test]
    fn package_workspace_stages_windows_exe_bundle_with_object_only_owner_attachment() {
        let dir = temp_dir("windows_exe_owner_object_only");
        write_app_workspace(
            &dir,
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

        let bundle = package_workspace(dir.clone(), BuildTarget::windows_exe(), None, None)
            .expect("package should succeed");
        let exe_path = bundle.bundle_dir.join(&bundle.root_artifact);
        let status = Command::new(&exe_path)
            .status()
            .expect("staged object-only owner bundle should launch");
        assert_eq!(status.code(), Some(5));

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
    #[test]
    fn package_workspace_stages_loadable_windows_dll_bundle() {
        let dir = temp_dir("windows_dll");
        write_std_text_grimoire(&dir);
        write_lib_workspace(
            &dir,
            concat!(
                "import std.text\n",
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
                "export fn prefix() -> Bytes:\n",
                "    return std.text.bytes_from_str_utf8 :: \"arc\" :: call\n",
                "export fn byte_len(read bytes: Bytes) -> Int:\n",
                "    return std.text.bytes_len :: bytes :: call\n",
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
            vec!["lib.dll.h".to_string(), "lib.dll.def".to_string()]
        );
        assert!(
            bundle.bundle_dir.join("lib.dll.def").is_file(),
            "expected staged dll definition file"
        );
        assert!(
            !bundle
                .bundle_dir
                .join("lib.dll.arcana-bundle.toml")
                .exists(),
            "did not expect staged dll native manifest"
        );
        assert!(
            !bundle.bundle_dir.join("arcana.bundle.toml").exists(),
            "did not expect staged distribution manifest beside dll"
        );
        let manifest = bundle
            .manifest_text
            .parse::<toml::Value>()
            .expect("distribution manifest should parse");
        let root_native_product = manifest
            .get("root_native_product")
            .and_then(toml::Value::as_table)
            .expect("root_native_product table should exist");
        assert_eq!(
            root_native_product
                .get("package_name")
                .and_then(toml::Value::as_str),
            Some("core")
        );
        assert_eq!(
            root_native_product
                .get("product_name")
                .and_then(toml::Value::as_str),
            Some("default")
        );
        assert_eq!(
            root_native_product
                .get("role")
                .and_then(toml::Value::as_str),
            Some("export")
        );
        assert_eq!(
            root_native_product
                .get("contract_id")
                .and_then(toml::Value::as_str),
            Some("arcana.cabi.export.v1")
        );
        assert_eq!(
            root_native_product
                .get("contract_version")
                .and_then(toml::Value::as_integer),
            Some(1)
        );
        assert_eq!(
            root_native_product
                .get("producer")
                .and_then(toml::Value::as_str),
            Some("arcana-source")
        );
        assert_eq!(
            root_native_product
                .get("file")
                .and_then(toml::Value::as_str),
            Some("lib.dll")
        );
        assert!(
            root_native_product
                .get("file_hash")
                .and_then(toml::Value::as_str)
                .is_some_and(|hash| hash.starts_with("sha256:")),
            "expected root dll hash in manifest: {}",
            bundle.manifest_text
        );
        compile_c_header_smoke(&bundle.bundle_dir, "lib.dll.h");

        unsafe {
            let library = Library::new(&dll_path).expect("dll should load");
            let answer = library
                .get::<unsafe extern "system" fn(*mut i64) -> u8>(b"answer")
                .expect("typed answer export should exist");
            let greet = library
                .get::<unsafe extern "system" fn(ArcanaViewV1, *mut ArcanaOwnedStr) -> u8>(b"greet")
                .expect("typed greet export should exist");
            let prefix = library
                .get::<unsafe extern "system" fn(*mut ArcanaOwnedBytes) -> u8>(b"prefix")
                .expect("typed prefix export should exist");
            let byte_len = library
                .get::<unsafe extern "system" fn(ArcanaViewV1, *mut i64) -> u8>(b"byte_len")
                .expect("typed byte_len export should exist");
            let echo_pair = library
                .get::<unsafe extern "system" fn(
                    ArcanaPairView__Pair__Str__Int,
                    *mut ArcanaPairOwned__Pair__Str__Int,
                ) -> u8>(b"echo_pair")
                .expect("typed pair export should exist");
            let last_error = library
                .get::<unsafe extern "system" fn(*mut usize) -> *mut u8>(
                    b"arcana_cabi_last_error_alloc_v1",
                )
                .expect("last-error export should exist");
            let free_bytes = library
                .get::<unsafe extern "system" fn(*mut u8, usize)>(
                    b"arcana_cabi_owned_bytes_free_v1",
                )
                .expect("byte free export should exist");
            let free_str = library
                .get::<unsafe extern "system" fn(*mut u8, usize)>(b"arcana_cabi_owned_str_free_v1")
                .expect("string free export should exist");
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
                ArcanaViewV1 {
                    ptr: name.as_ptr(),
                    len: name.len(),
                    stride_bytes: 1,
                    family: 1,
                    element_size: 1,
                    flags: 1,
                },
                &mut greeting,
            );
            if ok == 0 {
                let err =
                    read_allocated_utf8(&last_error, &free_bytes).expect("last error should read");
                panic!("typed greet export failed: {err}");
            }
            let greeting_text = read_owned_utf8(greeting, &free_str).expect("greeting utf8");
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
                ArcanaViewV1 {
                    ptr: payload.as_ptr(),
                    len: payload.len(),
                    stride_bytes: 1,
                    family: 1,
                    element_size: 1,
                    flags: 0,
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
                    left: ArcanaViewV1 {
                        ptr: pair_label.as_ptr(),
                        len: pair_label.len(),
                        stride_bytes: 1,
                        family: 1,
                        element_size: 1,
                        flags: 1,
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
                read_owned_utf8(echoed_pair.left, &free_str).expect("pair text should read");
            assert_eq!(echoed_left, "pair");
            assert_eq!(echoed_pair.right, 17);
        }

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
    #[test]
    fn package_workspace_stages_named_child_root_windows_dll_product() {
        let dir = temp_dir("windows_dll_child_root_product");
        write_file(
            &dir.join("book.toml"),
            concat!(
                "name = \"desktop\"\n",
                "kind = \"lib\"\n\n",
                "[native.products.default]\n",
                "kind = \"dll\"\n",
                "role = \"child\"\n",
                "producer = \"arcana-source\"\n",
                "file = \"desktop_runtime.dll\"\n",
                "contract = \"arcana.cabi.child.v1\"\n",
                "sidecars = []\n",
            ),
        );
        write_file(&dir.join("src/book.arc"), "fn touch():\n    return\n");
        write_file(&dir.join("src/types.arc"), "// types\n");

        let bundle = package_workspace_with_product(
            dir.clone(),
            BuildTarget::windows_dll(),
            Some("default".to_string()),
            None,
            None,
        )
        .expect("named child root product should package");
        assert_eq!(bundle.root_artifact, "desktop_runtime.dll");
        assert!(
            bundle.support_files.is_empty(),
            "root child product should not emit export support files: {:?}",
            bundle.support_files
        );
        let dll_path = bundle.bundle_dir.join(&bundle.root_artifact);
        assert!(
            dll_path.is_file(),
            "expected staged dll at {}",
            dll_path.display()
        );
        assert!(
            !bundle.bundle_dir.join("desktop_runtime.dll.h").exists(),
            "child root product should not emit export header support files"
        );
        assert!(
            !bundle.bundle_dir.join("desktop_runtime.dll.def").exists(),
            "child root product should not emit export definition support files"
        );
        assert!(
            !bundle.bundle_dir.join("arcana.bundle.toml").exists(),
            "did not expect staged distribution manifest beside root child product"
        );
        let manifest = bundle
            .manifest_text
            .parse::<toml::Value>()
            .expect("distribution manifest should parse");
        let root_native_product = manifest
            .get("root_native_product")
            .and_then(toml::Value::as_table)
            .expect("root_native_product table should exist");
        assert_eq!(
            root_native_product
                .get("package_name")
                .and_then(toml::Value::as_str),
            Some("desktop")
        );
        assert_eq!(
            root_native_product
                .get("product_name")
                .and_then(toml::Value::as_str),
            Some("default")
        );
        assert_eq!(
            root_native_product
                .get("role")
                .and_then(toml::Value::as_str),
            Some("child")
        );
        assert_eq!(
            root_native_product
                .get("contract_id")
                .and_then(toml::Value::as_str),
            Some("arcana.cabi.child.v1")
        );
        assert_eq!(
            root_native_product
                .get("contract_version")
                .and_then(toml::Value::as_integer),
            Some(1)
        );
        assert_eq!(
            root_native_product
                .get("producer")
                .and_then(toml::Value::as_str),
            Some("arcana-source")
        );
        assert_eq!(
            root_native_product
                .get("file")
                .and_then(toml::Value::as_str),
            Some("desktop_runtime.dll")
        );
        assert!(
            root_native_product
                .get("file_hash")
                .and_then(toml::Value::as_str)
                .is_some_and(|hash| hash.starts_with("sha256:")),
            "expected root child dll hash in manifest: {}",
            bundle.manifest_text
        );

        unsafe {
            let library = Library::new(&dll_path).expect("child dll should load");
            let get_api = library
                .get::<unsafe extern "system" fn() -> *const TestArcanaCabiProductApiV1>(
                    b"arcana_cabi_get_product_api_v1",
                )
                .expect("child product descriptor export should exist");
            let api = &*get_api();
            assert_eq!(read_cabi_utf8_field(api.package_name), "desktop");
            assert_eq!(read_cabi_utf8_field(api.product_name), "default");
            assert_eq!(read_cabi_utf8_field(api.role), "child");
            assert_eq!(
                read_cabi_utf8_field(api.contract_id),
                "arcana.cabi.child.v1"
            );
            assert!(
                !api.role_ops.is_null(),
                "child role_ops should be populated"
            );
        }

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
    #[test]
    fn package_workspace_rejects_invalid_rust_cdylib_native_product_descriptor() {
        let dir = temp_dir("windows_dll_invalid_rust_cdylib_descriptor");
        write_file(
            &dir.join("book.toml"),
            concat!(
                "name = \"desktop\"\n",
                "kind = \"lib\"\n\n",
                "[native.products.default]\n",
                "kind = \"dll\"\n",
                "role = \"plugin\"\n",
                "producer = \"rust-cdylib\"\n",
                "file = \"desktop_tools.dll\"\n",
                "contract = \"arcana.cabi.plugin.v1\"\n",
                "rust_cdylib_crate = \"plugin\"\n",
                "sidecars = []\n",
            ),
        );
        write_file(&dir.join("src/book.arc"), "fn touch():\n    return\n");
        write_file(&dir.join("src/types.arc"), "// types\n");
        write_file(
            &dir.join("plugin/Cargo.toml"),
            concat!(
                "[package]\n",
                "name = \"desktop_tools\"\n",
                "version = \"0.0.0\"\n",
                "edition = \"2021\"\n\n",
                "[lib]\n",
                "name = \"desktop_tools\"\n",
                "crate-type = [\"cdylib\"]\n\n",
                "[workspace]\n",
            ),
        );
        write_file(
            &dir.join("plugin/src/lib.rs"),
            concat!(
                "#[no_mangle]\n",
                "pub extern \"system\" fn unrelated_symbol() -> u8 {\n",
                "    1\n",
                "}\n",
            ),
        );

        let err = package_workspace_with_product(
            dir.clone(),
            BuildTarget::windows_dll(),
            Some("default".to_string()),
            None,
            None,
        )
        .expect_err("invalid rust-cdylib descriptor should fail packaging");
        assert!(
            err.contains("arcana_cabi_get_product_api_v1"),
            "unexpected error: {err}"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
    #[test]
    fn package_workspace_requires_product_for_non_export_windows_dll_roots() {
        let dir = temp_dir("windows_dll_child_root_requires_product");
        write_file(
            &dir.join("book.toml"),
            concat!(
                "name = \"desktop\"\n",
                "kind = \"lib\"\n\n",
                "[native.products.default]\n",
                "kind = \"dll\"\n",
                "role = \"child\"\n",
                "producer = \"arcana-source\"\n",
                "file = \"desktop_runtime.dll\"\n",
                "contract = \"arcana.cabi.child.v1\"\n",
                "sidecars = []\n",
            ),
        );
        write_file(&dir.join("src/book.arc"), "fn touch():\n    return\n");
        write_file(&dir.join("src/types.arc"), "// types\n");

        let err = package_workspace(dir.clone(), BuildTarget::windows_dll(), None, None)
            .expect_err("non-export root windows-dll build should require --product");
        assert!(
            err.contains("default export native product"),
            "unexpected error: {err}"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
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
                "    done: when Counter.value >= 4 retain [Counter]\n",
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
                .get::<unsafe extern "system" fn(*mut usize) -> *mut u8>(
                    b"arcana_cabi_last_error_alloc_v1",
                )
                .expect("last-error export should exist");
            let free_bytes = library
                .get::<unsafe extern "system" fn(*mut u8, usize)>(
                    b"arcana_cabi_owned_bytes_free_v1",
                )
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

    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
    #[test]
    fn package_workspace_runs_native_audio_app_bundle() {
        let dir = temp_dir("windows_audio");
        write_test_wav(&dir.join("fixture").join("clip.wav"));
        write_app_workspace(
            &dir,
            concat!(
                "import arcana_audio\n",
                "import arcana_process.io\n",
                "use std.result.Result\n",
                "fn use_playback(take device: arcana_audio.types.AudioDevice, take playback: arcana_audio.types.AudioPlayback) -> Int:\n",
                "    let stop = playback :: :: stop\n",
                "    if stop :: :: is_err:\n",
                "        return 4\n",
                "    let close = arcana_audio.output.close :: device :: call\n",
                "    if close :: :: is_err:\n",
                "        return 5\n",
                "    return 0\n",
                "fn use_clip(take device: arcana_audio.types.AudioDevice, read clip: arcana_audio.types.AudioBuffer) -> Int:\n",
                "    let mut device = device\n",
                "    arcana_process.io.print[Int] :: ((arcana_audio.clip.info :: clip :: call).sample_rate_hz) :: call\n",
                "    let playback_result = arcana_audio.playback.play :: device, clip :: call\n",
                "    return match playback_result:\n",
                "        Result.Ok(value) => use_playback :: device, value :: call\n",
                "        Result.Err(_) => 3\n",
                "fn main() -> Int:\n",
                "    return match (arcana_audio.output.default_output :: :: call):\n",
                "        Result.Ok(device) => match (arcana_audio.clip.load_wav :: \"clip.wav\" :: call):\n",
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

    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
    #[test]
    fn package_workspace_rejects_native_audio_buffer_format_mismatch() {
        let dir = temp_dir("windows_audio_mismatch");
        write_test_wav_with_format(&dir.join("fixture").join("clip_48k_stereo.wav"), 48_000, 2);
        write_test_wav_with_format(&dir.join("fixture").join("clip_44k_stereo.wav"), 44_100, 2);
        write_app_workspace(
            &dir,
            concat!(
                "import arcana_audio\n",
                "use std.result.Result\n",
                "fn mismatch_path(read device: arcana_audio.types.AudioDevice) -> Str:\n",
                "    if (arcana_audio.output.sample_rate_hz :: device :: call) == 48000:\n",
                "        return \"clip_44k_stereo.wav\"\n",
                "    return \"clip_48k_stereo.wav\"\n",
                "fn use_device(take device: arcana_audio.types.AudioDevice) -> Int:\n",
                "    let mut device = device\n",
                "    let path = mismatch_path :: device :: call\n",
                "    return match (arcana_audio.clip.load_wav :: path :: call):\n",
                "        Result.Ok(clip) => match (arcana_audio.playback.play :: device, clip :: call):\n",
                "            Result.Ok(_) => 4\n",
                "            Result.Err(_) => 0\n",
                "        Result.Err(_) => 3\n",
                "fn main() -> Int:\n",
                "    return match (arcana_audio.output.default_output :: :: call):\n",
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

    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
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

    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
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

    #[cfg(all(windows, feature = "windows-native-bundle-tests"))]
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

