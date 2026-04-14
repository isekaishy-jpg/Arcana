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

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    use std::ffi::{CStr, c_char, c_void};
    use std::fs;
    use std::io::Read;
    use std::ops::{Deref, DerefMut};
    use std::path::Path;
    use std::thread;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    use libloading::Library;
    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    use std::process::{Child, Command, Stdio};
    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    type HWND = *mut c_void;

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    const WM_CLOSE: u32 = 0x0010;
    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    const WM_MOUSEMOVE: u32 = 0x0200;
    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    const WM_LBUTTONDOWN: u32 = 0x0201;
    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    const WM_LBUTTONUP: u32 = 0x0202;
    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    const WM_CHAR: u32 = 0x0102;
    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    const WM_IME_STARTCOMPOSITION: u32 = 0x010D;
    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    const WM_IME_ENDCOMPOSITION: u32 = 0x010E;

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
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

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    struct WindowSearch {
        pid: u32,
        title_contains: String,
        hwnd: HWND,
        title: String,
    }

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    struct WindowListSearch {
        pid: u32,
        windows: Vec<(HWND, String)>,
    }

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    struct TestChild {
        child: Child,
    }

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    impl TestChild {
        fn new(child: Child) -> Self {
            Self { child }
        }
    }

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    impl Deref for TestChild {
        type Target = Child;

        fn deref(&self) -> &Self::Target {
            &self.child
        }
    }

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    impl DerefMut for TestChild {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.child
        }
    }

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    impl Drop for TestChild {
        fn drop(&mut self) {
            if matches!(self.child.try_wait(), Ok(None)) {
                let _ = self.child.kill();
                let _ = self.child.wait();
            }
        }
    }

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
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

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
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

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    fn wait_for_process_window(pid: u32, timeout: Duration) -> Option<(HWND, String)> {
        let start = Instant::now();
        while start.elapsed() < timeout {
            let mut search = WindowSearch {
                pid,
                title_contains: String::new(),
                hwnd: std::ptr::null_mut(),
                title: String::new(),
            };
            unsafe {
                EnumWindows(
                    Some(collect_process_window),
                    &mut search as *mut WindowSearch as isize,
                );
            }
            if !search.hwnd.is_null() {
                return Some((search.hwnd, search.title));
            }
            thread::sleep(Duration::from_millis(25));
        }
        None
    }

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    fn process_windows(pid: u32) -> Vec<(HWND, String)> {
        let mut search = WindowListSearch {
            pid,
            windows: Vec::new(),
        };
        unsafe {
            EnumWindows(
                Some(collect_process_windows),
                &mut search as *mut WindowListSearch as isize,
            );
        }
        search.windows
    }

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    fn wait_for_named_process_window(
        pid: u32,
        title_contains: &str,
        timeout: Duration,
    ) -> Option<(HWND, String)> {
        let start = Instant::now();
        while start.elapsed() < timeout {
            if let Some(found) = process_windows(pid)
                .into_iter()
                .find(|(_, title)| title.contains(title_contains))
            {
                return Some(found);
            }
            thread::sleep(Duration::from_millis(25));
        }
        None
    }

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    fn wait_for_additional_named_process_window(
        pid: u32,
        exclude: HWND,
        title_contains: &str,
        timeout: Duration,
    ) -> Option<(HWND, String)> {
        let start = Instant::now();
        while start.elapsed() < timeout {
            if let Some(found) = process_windows(pid)
                .into_iter()
                .find(|(hwnd, title)| *hwnd != exclude && title.contains(title_contains))
            {
                return Some(found);
            }
            thread::sleep(Duration::from_millis(25));
        }
        None
    }

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    fn read_window_title(hwnd: HWND) -> String {
        let title_len = unsafe { GetWindowTextLengthW(hwnd) };
        if title_len <= 0 {
            return String::new();
        }
        let mut buffer = vec![0u16; usize::try_from(title_len).unwrap_or(0) + 1];
        let read = unsafe { GetWindowTextW(hwnd, buffer.as_mut_ptr(), title_len + 1) };
        if read <= 0 {
            return String::new();
        }
        String::from_utf16_lossy(&buffer[..usize::try_from(read).unwrap_or(0)])
    }

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    fn wait_for_window_title_contains(
        hwnd: HWND,
        needle: &str,
        timeout: Duration,
    ) -> Option<String> {
        let start = Instant::now();
        while start.elapsed() < timeout {
            let title = read_window_title(hwnd);
            if title.contains(needle) {
                return Some(title);
            }
            thread::sleep(Duration::from_millis(25));
        }
        None
    }

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    fn click_until_window_gone(
        pid: u32,
        click_hwnd: HWND,
        x: i32,
        y: i32,
        target_hwnd: HWND,
        timeout: Duration,
    ) -> Vec<(HWND, String)> {
        let start = Instant::now();
        let mut windows = process_windows(pid);
        let mut last_click = Instant::now()
            .checked_sub(Duration::from_millis(200))
            .unwrap_or_else(Instant::now);
        while start.elapsed() < timeout {
            windows = process_windows(pid);
            if windows.iter().all(|(hwnd, _)| *hwnd != target_hwnd) {
                break;
            }
            if last_click.elapsed() >= Duration::from_millis(200) {
                send_left_click(click_hwnd, x, y);
                last_click = Instant::now();
            }
            thread::sleep(Duration::from_millis(50));
        }
        windows
    }

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    fn pack_mouse_lparam(x: i32, y: i32) -> isize {
        ((y as u32) << 16 | (x as u32 & 0xFFFF)) as isize
    }

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    fn send_left_click(hwnd: HWND, x: i32, y: i32) {
        unsafe {
            SetForegroundWindow(hwnd);
            let lparam = pack_mouse_lparam(x, y);
            SendMessageW(hwnd, WM_MOUSEMOVE, 0, lparam);
            SendMessageW(hwnd, WM_LBUTTONDOWN, 0, lparam);
            SendMessageW(hwnd, WM_LBUTTONUP, 0, lparam);
        }
    }

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    fn desktop_showcase_button_center(id: i32) -> (i32, i32) {
        let width = 1280;
        let gutter = 18;
        let available_width = width - gutter * 4;
        let left_width = (available_width * 30 / 100).clamp(320, 420);
        let inner_button_width = left_width - gutter * 2;
        let button_cols = 3;
        let button_gap_x = 8;
        let button_gap_y = 8;
        let button_width = (inner_button_width - (button_cols - 1) * button_gap_x) / button_cols;
        let button_height = 30;
        let col = id % button_cols;
        let row = id / button_cols;
        let x = gutter + gutter + col * (button_width + button_gap_x) + button_width / 2;
        let y = 84 + gutter + row * (button_height + button_gap_y) + button_height / 2;
        (x, y)
    }

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    fn wait_for_child_exit(
        child: &mut Child,
        timeout: Duration,
    ) -> Option<std::process::ExitStatus> {
        let start = Instant::now();
        while start.elapsed() < timeout {
            if let Ok(Some(status)) = child.try_wait() {
                return Some(status);
            }
            thread::sleep(Duration::from_millis(25));
        }
        None
    }

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    fn drive_native_text_input_and_ime(hwnd: HWND) -> Result<(), String> {
        unsafe {
            SetForegroundWindow(hwnd);
            SendMessageW(hwnd, WM_IME_STARTCOMPOSITION, 0, 0);
            SendMessageW(hwnd, WM_IME_ENDCOMPOSITION, 0, 0);
            SendMessageW(hwnd, WM_CHAR, 'x' as usize, 0);
        }
        Ok(())
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

    fn desktop_proof_workspace_source_dir() -> PathBuf {
        repo_root().join("examples").join("arcana-desktop-proof")
    }

    fn should_skip_workspace_copy(path: &Path) -> bool {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == ".arcana" || name == "dist" || name == "Arcana.lock")
    }

    fn copy_dir_filtered(src: &Path, dst: &Path) {
        fs::create_dir_all(dst).expect("copy target dir should exist");
        for entry in fs::read_dir(src).expect("source dir should be readable") {
            let entry = entry.expect("dir entry should read");
            let src_path = entry.path();
            if should_skip_workspace_copy(&src_path) {
                continue;
            }
            let dst_path = dst.join(entry.file_name());
            if src_path.is_dir() {
                copy_dir_filtered(&src_path, &dst_path);
            } else {
                fs::copy(&src_path, &dst_path).unwrap_or_else(|err| {
                    panic!(
                        "failed to copy {} to {}: {err}",
                        src_path.display(),
                        dst_path.display()
                    )
                });
            }
        }
    }

    fn desktop_proof_workspace_copy(label: &str) -> PathBuf {
        let dir = repo_temp_dir(label);
        copy_dir_filtered(&desktop_proof_workspace_source_dir(), &dir);
        dir
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

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
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

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
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

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
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

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
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

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
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

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct ArcanaOwnedStr {
        ptr: *mut u8,
        len: usize,
    }

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct ArcanaOwnedBytes {
        ptr: *mut u8,
        len: usize,
    }

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct ArcanaPairView__Pair__Str__Int {
        left: ArcanaViewV1,
        right: i64,
    }

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct ArcanaPairOwned__Pair__Str__Int {
        left: ArcanaOwnedStr,
        right: i64,
    }

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
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

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    unsafe fn read_cabi_utf8_field(ptr: *const c_char) -> String {
        unsafe { CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned()
    }

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
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

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
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

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    #[test]
    fn package_workspace_runs_arcana_desktop_windows_exe_bundle() {
        let dir = temp_dir("windows_exe_arcana_desktop");
        let desktop_dep = repo_root()
            .join("grimoires")
            .join("owned")
            .join("libs")
            .join("arcana-desktop")
            .to_string_lossy()
            .replace('\\', "/");
        write_file(
            &dir.join("book.toml"),
            &format!(
                concat!(
                    "name = \"app\"\n",
                    "kind = \"app\"\n",
                    "[deps]\n",
                    "arcana_desktop = {desktop_dep:?}\n",
                ),
                desktop_dep = desktop_dep,
            ),
        );
        write_file(
            &dir.join("src/shelf.arc"),
            concat!(
                "import arcana_desktop.app\n",
                "import arcana_desktop.events\n",
                "import arcana_desktop.monitor\n",
                "import arcana_desktop.types\n",
                "import arcana_desktop.window\n",
                "import arcana_process.io\n",
                "\n",
                "record Demo:\n",
                "    seen: Int\n",
                "\n",
                "impl arcana_desktop.app.Application[Demo] for Demo:\n",
                "    fn resumed(edit self: Demo, edit cx: arcana_desktop.types.AppContext):\n",
                "        let mut main_window = (arcana_desktop.app.main_window_or_cached :: cx :: call)\n",
                "        let scale = arcana_desktop.window.scale_factor_milli :: main_window :: call\n",
                "        let current = arcana_desktop.monitor.current :: main_window :: call\n",
                "        let primary = arcana_desktop.monitor.primary :: :: call\n",
                "        let count = arcana_desktop.monitor.count :: :: call\n",
                "        self.seen = 0\n",
                "        if scale > 0:\n",
                "            self.seen += 1\n",
                "        if count >= 1:\n",
                "            self.seen += 1\n",
                "        if current.scale_factor_milli > 0:\n",
                "            self.seen += 1\n",
                "        if primary.primary:\n",
                "            self.seen += 1\n",
                "        self.seen += theme_score :: (arcana_desktop.window.theme :: main_window :: call) :: call\n",
                "        arcana_desktop.window.request_attention :: main_window, false :: call\n",
                "        arcana_desktop.events.wake :: (arcana_desktop.app.wake_handle :: cx :: call) :: call\n",
                "        arcana_desktop.app.set_control_flow :: cx, (arcana_desktop.types.ControlFlow.Wait :: :: call) :: call\n",
                "\n",
                "    fn suspended(edit self: Demo, edit cx: arcana_desktop.types.AppContext):\n",
                "        return\n",
                "\n",
                "    fn window_event(edit self: Demo, edit cx: arcana_desktop.types.AppContext, read target: arcana_desktop.types.TargetedEvent) -> arcana_desktop.types.ControlFlow:\n",
                "        return match target.event:\n",
                "            arcana_desktop.types.WindowEvent.WindowRedrawRequested(id) => on_redraw :: self, cx, id :: call\n",
                "            _ => cx.control.control_flow\n",
                "\n",
                "    fn device_event(edit self: Demo, edit cx: arcana_desktop.types.AppContext, read event: arcana_desktop.types.DeviceEvent) -> arcana_desktop.types.ControlFlow:\n",
                "        return cx.control.control_flow\n",
                "\n",
                "    fn about_to_wait(edit self: Demo, edit cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.ControlFlow:\n",
                "        return cx.control.control_flow\n",
                "\n",
                "    fn wake(edit self: Demo, edit cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.ControlFlow:\n",
                "        let mut main_window = (arcana_desktop.app.main_window_or_cached :: cx :: call)\n",
                "        arcana_desktop.app.request_window_redraw :: cx, main_window :: call\n",
                "        return arcana_desktop.types.ControlFlow.Wait :: :: call\n",
                "\n",
                "    fn exiting(edit self: Demo, edit cx: arcana_desktop.types.AppContext):\n",
                "        return\n",
                "\n",
                "fn theme_score(read theme: arcana_desktop.types.WindowTheme) -> Int:\n",
                "    return match theme:\n",
                "        arcana_desktop.types.WindowTheme.Light => 1\n",
                "        arcana_desktop.types.WindowTheme.Dark => 1\n",
                "        arcana_desktop.types.WindowTheme.Unknown => 1\n",
                "\n",
                "fn on_redraw(edit self: Demo, edit cx: arcana_desktop.types.AppContext, id: Int) -> arcana_desktop.types.ControlFlow:\n",
                "    arcana_process.io.print_line[Int] :: id :: call\n",
                "    arcana_process.io.print_line[Int] :: self.seen :: call\n",
                "    arcana_desktop.app.request_exit :: cx, 0 :: call\n",
                "    return arcana_desktop.types.ControlFlow.Wait :: :: call\n",
                "\n",
                "fn main() -> Int:\n",
                "    let mut app = Demo :: seen = 0 :: call\n",
                "    return arcana_desktop.app.run :: app, (arcana_desktop.app.default_app_config :: :: call) :: call\n",
            ),
        );
        write_file(&dir.join("src/types.arc"), "// types\n");

        let bundle = package_workspace(dir.clone(), BuildTarget::windows_exe(), None, None)
            .expect("package should succeed");
        let exe_path = bundle.bundle_dir.join(&bundle.root_artifact);
        let output = Command::new(&exe_path)
            .output()
            .expect("staged desktop bundle should launch");
        assert_eq!(output.status.code(), Some(0));
        assert_eq!(
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .collect::<Vec<_>>(),
            vec!["0", "5"]
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    #[test]
    fn package_workspace_runs_arcana_desktop_multi_window_clipboard_windows_exe_bundle() {
        let dir = temp_dir("windows_exe_arcana_desktop_multi_window_clipboard");
        let desktop_dep = repo_root()
            .join("grimoires")
            .join("libs")
            .join("arcana-desktop")
            .to_string_lossy()
            .replace('\\', "/");
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
                    "name = \"app\"\n",
                    "kind = \"app\"\n",
                    "[deps]\n",
                    "arcana_desktop = {desktop_dep:?}\n",
                    "arcana_process = {process_dep:?}\n",
                    "arcana_winapi = {winapi_dep:?}\n",
                ),
                desktop_dep = desktop_dep,
                process_dep = process_dep,
                winapi_dep = winapi_dep,
            ),
        );
        write_file(
            &dir.join("src/shelf.arc"),
            concat!(
                "import arcana_desktop.app\n",
                "import arcana_desktop.clipboard\n",
                "import arcana_desktop.types\n",
                "import arcana_desktop.window\n",
                "import arcana_winapi.desktop_handles\n",
                "import std.result\n",
                "import arcana_process.io\n",
                "\n",
                "record Demo:\n",
                "    second_window: Int\n",
                "\n",
                "impl arcana_desktop.app.Application[Demo] for Demo:\n",
                "    fn resumed(edit self: Demo, edit cx: arcana_desktop.types.AppContext):\n",
                "        let wrote = arcana_desktop.clipboard.write_text :: \"desk\" :: call\n",
                "        if wrote :: :: is_err:\n",
                "            arcana_desktop.app.request_exit :: cx, 7 :: call\n",
                "            return\n",
                "        let text = (arcana_desktop.clipboard.read_text :: :: call) :: \"\" :: unwrap_or\n",
                "        if text != \"desk\":\n",
                "            arcana_desktop.app.request_exit :: cx, 8 :: call\n",
                "            return\n",
                "        let opened = arcana_desktop.app.open_window :: cx, \"Second\", (160, 120) :: call\n",
                "        return match opened:\n",
                "            std.result.Result.Ok(win) => on_second_window :: self, cx, win :: call\n",
                "            std.result.Result.Err(_) => arcana_desktop.app.request_exit :: cx, 9 :: call\n",
                "\n",
                "    fn suspended(edit self: Demo, edit cx: arcana_desktop.types.AppContext):\n",
                "        return\n",
                "\n",
                "    fn window_event(edit self: Demo, edit cx: arcana_desktop.types.AppContext, read target: arcana_desktop.types.TargetedEvent) -> arcana_desktop.types.ControlFlow:\n",
                "        return match target.event:\n",
                "            arcana_desktop.types.WindowEvent.WindowRedrawRequested(id) => on_redraw :: self, cx, id :: call\n",
                "            _ => cx.control.control_flow\n",
                "\n",
                "    fn device_event(edit self: Demo, edit cx: arcana_desktop.types.AppContext, read event: arcana_desktop.types.DeviceEvent) -> arcana_desktop.types.ControlFlow:\n",
                "        return cx.control.control_flow\n",
                "\n",
                "    fn about_to_wait(edit self: Demo, edit cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.ControlFlow:\n",
                "        return cx.control.control_flow\n",
                "\n",
                "    fn wake(edit self: Demo, edit cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.ControlFlow:\n",
                "        return cx.control.control_flow\n",
                "\n",
                "    fn exiting(edit self: Demo, edit cx: arcana_desktop.types.AppContext):\n",
                "        return\n",
                "\n",
                "fn on_second_window(edit self: Demo, edit cx: arcana_desktop.types.AppContext, take win: arcana_winapi.desktop_handles.Window):\n",
                "    let mut win = win\n",
                "    self.second_window = (arcana_desktop.window.id :: win :: call).value\n",
                "    arcana_desktop.app.request_window_redraw :: cx, win :: call\n",
                "    arcana_desktop.app.set_control_flow :: cx, (arcana_desktop.types.ControlFlow.Wait :: :: call) :: call\n",
                "    return\n",
                "\n",
                "fn on_redraw(edit self: Demo, edit cx: arcana_desktop.types.AppContext, id: Int) -> arcana_desktop.types.ControlFlow:\n",
                "    if id == self.second_window:\n",
                "        arcana_process.io.print_line[Int] :: id :: call\n",
                "        arcana_desktop.app.request_exit :: cx, 0 :: call\n",
                "    return arcana_desktop.types.ControlFlow.Wait :: :: call\n",
                "\n",
                "fn main() -> Int:\n",
                "    let mut app = Demo :: second_window = -1 :: call\n",
                "    return arcana_desktop.app.run :: app, (arcana_desktop.app.default_app_config :: :: call) :: call\n",
            ),
        );
        write_file(&dir.join("src/types.arc"), "// types\n");

        let bundle = package_workspace(dir.clone(), BuildTarget::windows_exe(), None, None)
            .expect("package should succeed");
        let exe_path = bundle.bundle_dir.join(&bundle.root_artifact);
        let output = Command::new(&exe_path)
            .output()
            .expect("staged desktop multi-window clipboard bundle should launch");
        assert_eq!(
            output.status.code(),
            Some(0),
            "stdout=`{}` stderr=`{}`",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .collect::<Vec<_>>(),
            vec!["1"]
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    #[test]
    fn package_workspace_runs_arcana_desktop_settings_windows_exe_bundle() {
        let dir = temp_dir("windows_exe_arcana_desktop_settings");
        let desktop_dep = repo_root()
            .join("grimoires")
            .join("owned")
            .join("libs")
            .join("arcana-desktop")
            .to_string_lossy()
            .replace('\\', "/");
        write_file(
            &dir.join("book.toml"),
            &format!(
                concat!(
                    "name = \"app\"\n",
                    "kind = \"app\"\n",
                    "[deps]\n",
                    "arcana_desktop = {desktop_dep:?}\n",
                ),
                desktop_dep = desktop_dep,
            ),
        );
        write_file(
            &dir.join("src/shelf.arc"),
            concat!(
                "import arcana_desktop.app\n",
                "import arcana_desktop.text_input\n",
                "import arcana_desktop.types\n",
                "import arcana_desktop.window\n",
                "import arcana_process.io\n",
                "\n",
                "record Demo:\n",
                "    done: Bool\n",
                "\n",
                "impl arcana_desktop.app.Application[Demo] for Demo:\n",
                "    fn resumed(edit self: Demo, edit cx: arcana_desktop.types.AppContext):\n",
                "        let mut main_window = (arcana_desktop.app.main_window_or_cached :: cx :: call)\n",
                "        arcana_desktop.window.set_min_size :: main_window, 111, 112 :: call\n",
                "        arcana_desktop.window.set_max_size :: main_window, 333, 334 :: call\n",
                "        arcana_desktop.window.set_transparent :: main_window, true :: call\n",
                "        arcana_desktop.window.set_theme_override :: main_window, (arcana_desktop.types.WindowThemeOverride.Dark :: :: call) :: call\n",
                "        arcana_desktop.window.set_cursor_icon :: main_window, (arcana_desktop.types.CursorIcon.Hand :: :: call) :: call\n",
                "        arcana_desktop.window.set_text_input_enabled :: main_window, false :: call\n",
                "        arcana_desktop.text_input.set_enabled :: main_window, true :: call\n",
                "        let area = arcana_desktop.types.CompositionArea :: active = true, position = (9, 10), size = (20, 21) :: call\n",
                "        arcana_desktop.text_input.set_composition_area :: main_window, area :: call\n",
                "        let current = arcana_desktop.window.settings :: main_window :: call\n",
                "        let text = arcana_desktop.text_input.settings :: main_window :: call\n",
                "        let mut total = 0\n",
                "        if current.bounds.min_size.0 == 111:\n",
                "            if current.bounds.min_size.1 == 112:\n",
                "                total += 1\n",
                "        if current.bounds.max_size.0 == 333:\n",
                "            if current.bounds.max_size.1 == 334:\n",
                "                total += 2\n",
                "        if current.options.style.transparent:\n",
                "            total += 4\n",
                "        if current.options.state.theme_override == (arcana_desktop.types.WindowThemeOverride.Dark :: :: call):\n",
                "            total += 8\n",
                "        if current.options.cursor.icon == (arcana_desktop.types.CursorIcon.Hand :: :: call):\n",
                "            total += 16\n",
                "        if text.enabled:\n",
                "            total += 32\n",
                "        if text.composition_area.active:\n",
                "            if text.composition_area.position.0 == 9:\n",
                "                if text.composition_area.position.1 == 10:\n",
                "                    if text.composition_area.size.0 == 20:\n",
                "                        if text.composition_area.size.1 == 21:\n",
                "                            total += 64\n",
                "        arcana_process.io.print_line[Int] :: total :: call\n",
                "        self.done = true\n",
                "        arcana_desktop.app.request_exit :: cx, 0 :: call\n",
                "\n",
                "    fn suspended(edit self: Demo, edit cx: arcana_desktop.types.AppContext):\n",
                "        return\n",
                "\n",
                "    fn window_event(edit self: Demo, edit cx: arcana_desktop.types.AppContext, read target: arcana_desktop.types.TargetedEvent) -> arcana_desktop.types.ControlFlow:\n",
                "        return cx.control.control_flow\n",
                "\n",
                "    fn device_event(edit self: Demo, edit cx: arcana_desktop.types.AppContext, read event: arcana_desktop.types.DeviceEvent) -> arcana_desktop.types.ControlFlow:\n",
                "        return cx.control.control_flow\n",
                "\n",
                "    fn about_to_wait(edit self: Demo, edit cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.ControlFlow:\n",
                "        return cx.control.control_flow\n",
                "\n",
                "    fn wake(edit self: Demo, edit cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.ControlFlow:\n",
                "        return cx.control.control_flow\n",
                "\n",
                "    fn exiting(edit self: Demo, edit cx: arcana_desktop.types.AppContext):\n",
                "        return\n",
                "\n",
                "fn main() -> Int:\n",
                "    let mut app = Demo :: done = false :: call\n",
                "    return arcana_desktop.app.run :: app, (arcana_desktop.app.default_app_config :: :: call) :: call\n",
            ),
        );
        write_file(&dir.join("src/types.arc"), "// types\n");

        let bundle = package_workspace(dir.clone(), BuildTarget::windows_exe(), None, None)
            .expect("package should succeed");
        let exe_path = bundle.bundle_dir.join(&bundle.root_artifact);
        let output = Command::new(&exe_path)
            .output()
            .expect("staged desktop settings bundle should launch");
        assert_eq!(output.status.code(), Some(0));
        assert_eq!(
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .collect::<Vec<_>>(),
            vec!["127"]
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    #[test]
    fn package_workspace_runs_arcana_desktop_record_settings_windows_exe_bundle() {
        let dir = temp_dir("windows_exe_arcana_desktop_record_settings");
        let desktop_dep = repo_root()
            .join("grimoires")
            .join("owned")
            .join("libs")
            .join("arcana-desktop")
            .to_string_lossy()
            .replace('\\', "/");
        write_file(
            &dir.join("book.toml"),
            &format!(
                concat!(
                    "name = \"app\"\n",
                    "kind = \"app\"\n",
                    "[deps]\n",
                    "arcana_desktop = {desktop_dep:?}\n",
                ),
                desktop_dep = desktop_dep,
            ),
        );
        write_file(
            &dir.join("src/shelf.arc"),
            concat!(
                "import arcana_desktop.app\n",
                "import arcana_desktop.text_input\n",
                "import arcana_desktop.types\n",
                "import arcana_desktop.window\n",
                "import arcana_process.io\n",
                "\n",
                "record Demo:\n",
                "    done: Bool\n",
                "\n",
                "impl arcana_desktop.app.Application[Demo] for Demo:\n",
                "    fn resumed(edit self: Demo, edit cx: arcana_desktop.types.AppContext):\n",
                "        let mut main_window = (arcana_desktop.app.main_window_or_cached :: cx :: call)\n",
                "        let mut settings = arcana_desktop.window.settings :: main_window :: call\n",
                "        settings.title = \"Applied\"\n",
                "        settings.bounds.min_size = (900, 620)\n",
                "        settings.bounds.max_size = (1540, 1040)\n",
                "        settings.options.style.transparent = true\n",
                "        settings.options.state.theme_override = (arcana_desktop.types.WindowThemeOverride.Dark :: :: call)\n",
                "        settings.options.cursor.icon = (arcana_desktop.types.CursorIcon.Hand :: :: call)\n",
                "        settings.options.cursor.position = (160, 128)\n",
                "        settings.options.text_input_enabled = true\n",
                "        arcana_desktop.window.apply_settings :: main_window, settings :: call\n",
                "        let mut text = arcana_desktop.text_input.settings :: main_window :: call\n",
                "        text.enabled = true\n",
                "        text.composition_area.active = true\n",
                "        text.composition_area.position = (120, 540)\n",
                "        text.composition_area.size = (260, 28)\n",
                "        arcana_desktop.text_input.apply_settings :: main_window, text :: call\n",
                "        let current = arcana_desktop.window.settings :: main_window :: call\n",
                "        let text_now = arcana_desktop.text_input.settings :: main_window :: call\n",
                "        let mut total = 0\n",
                "        if current.bounds.min_size.0 == 900:\n",
                "            if current.bounds.min_size.1 == 620:\n",
                "                total += 1\n",
                "        if current.bounds.max_size.0 == 1540:\n",
                "            if current.bounds.max_size.1 == 1040:\n",
                "                total += 2\n",
                "        if current.options.style.transparent:\n",
                "            total += 4\n",
                "        if current.options.state.theme_override == (arcana_desktop.types.WindowThemeOverride.Dark :: :: call):\n",
                "            total += 8\n",
                "        if current.options.cursor.icon == (arcana_desktop.types.CursorIcon.Hand :: :: call):\n",
                "            total += 16\n",
                "        if current.options.cursor.position.0 == 160:\n",
                "            if current.options.cursor.position.1 == 128:\n",
                "                total += 32\n",
                "        if text_now.enabled:\n",
                "            if text_now.composition_area.active:\n",
                "                if text_now.composition_area.position.0 == 120:\n",
                "                    if text_now.composition_area.position.1 == 540:\n",
                "                        if text_now.composition_area.size.0 == 260:\n",
                "                            if text_now.composition_area.size.1 == 28:\n",
                "                                total += 64\n",
                "        arcana_process.io.print_line[Int] :: total :: call\n",
                "        self.done = true\n",
                "        arcana_desktop.app.request_exit :: cx, 0 :: call\n",
                "\n",
                "    fn suspended(edit self: Demo, edit cx: arcana_desktop.types.AppContext):\n",
                "        return\n",
                "\n",
                "    fn window_event(edit self: Demo, edit cx: arcana_desktop.types.AppContext, read target: arcana_desktop.types.TargetedEvent) -> arcana_desktop.types.ControlFlow:\n",
                "        return cx.control.control_flow\n",
                "\n",
                "    fn device_event(edit self: Demo, edit cx: arcana_desktop.types.AppContext, read event: arcana_desktop.types.DeviceEvent) -> arcana_desktop.types.ControlFlow:\n",
                "        return cx.control.control_flow\n",
                "\n",
                "    fn about_to_wait(edit self: Demo, edit cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.ControlFlow:\n",
                "        return cx.control.control_flow\n",
                "\n",
                "    fn wake(edit self: Demo, edit cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.ControlFlow:\n",
                "        return cx.control.control_flow\n",
                "\n",
                "    fn exiting(edit self: Demo, edit cx: arcana_desktop.types.AppContext):\n",
                "        return\n",
                "\n",
                "fn main() -> Int:\n",
                "    let mut app = Demo :: done = false :: call\n",
                "    return arcana_desktop.app.run :: app, (arcana_desktop.app.default_app_config :: :: call) :: call\n",
            ),
        );
        write_file(&dir.join("src/types.arc"), "// types\n");

        let bundle = package_workspace(dir.clone(), BuildTarget::windows_exe(), None, None)
            .expect("package should succeed");
        let exe_path = bundle.bundle_dir.join(&bundle.root_artifact);
        let output = Command::new(&exe_path)
            .output()
            .expect("staged desktop record-settings bundle should launch");
        assert_eq!(output.status.code(), Some(0));
        assert_eq!(
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .collect::<Vec<_>>(),
            vec!["127"]
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    #[test]
    fn package_workspace_runs_arcana_desktop_text_input_ime_windows_exe_bundle() {
        let dir = temp_dir("windows_exe_arcana_desktop_text_input_ime");
        let desktop_dep = repo_root()
            .join("grimoires")
            .join("owned")
            .join("libs")
            .join("arcana-desktop")
            .to_string_lossy()
            .replace('\\', "/");
        let title = format!(
            "Arcana Desktop IME {}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after epoch")
                .as_nanos()
        );
        write_file(
            &dir.join("book.toml"),
            &format!(
                concat!(
                    "name = \"app\"\n",
                    "kind = \"app\"\n",
                    "[deps]\n",
                    "arcana_desktop = {desktop_dep:?}\n",
                ),
                desktop_dep = desktop_dep,
            ),
        );
        write_file(
            &dir.join("src/shelf.arc"),
            &format!(
                concat!(
                    "import arcana_desktop.app\n",
                    "import arcana_desktop.text_input\n",
                    "import arcana_desktop.types\n",
                    "import arcana_desktop.window\n",
                    "import arcana_process.io\n",
                    "\n",
                    "record Demo:\n",
                    "    settings_ok: Bool\n",
                    "    saw_text: Bool\n",
                    "    saw_comp_started: Bool\n",
                    "    saw_comp_cancelled: Bool\n",
                    "    done: Bool\n",
                    "\n",
                    "fn default_demo() -> Demo:\n",
                    "    let mut demo = Demo :: settings_ok = false, saw_text = false, saw_comp_started = false :: call\n",
                    "    demo.saw_comp_cancelled = false\n",
                    "    demo.done = false\n",
                    "    return demo\n",
                    "\n",
                    "fn finish_if_ready(edit self: Demo, edit cx: arcana_desktop.types.AppContext):\n",
                    "    if self.done:\n",
                    "        return\n",
                    "    if self.saw_comp_cancelled:\n",
                    "        if self.settings_ok:\n",
                    "            if self.saw_text:\n",
                    "                if self.saw_comp_started:\n",
                    "                    self.done = true\n",
                    "                    arcana_process.io.print_line[Int] :: 1 :: call\n",
                    "                    arcana_desktop.app.request_exit :: cx, 0 :: call\n",
                    "\n",
                    "impl arcana_desktop.app.Application[Demo] for Demo:\n",
                    "    fn resumed(edit self: Demo, edit cx: arcana_desktop.types.AppContext):\n",
                    "        let mut main_window = (arcana_desktop.app.main_window_or_cached :: cx :: call)\n",
                    "        arcana_desktop.text_input.set_enabled :: main_window, true :: call\n",
                    "        let area = arcana_desktop.types.CompositionArea :: active = true, position = (9, 10), size = (20, 21) :: call\n",
                    "        arcana_desktop.text_input.set_composition_area :: main_window, area :: call\n",
                    "        let current = arcana_desktop.window.settings :: main_window :: call\n",
                    "        let text = arcana_desktop.text_input.settings :: main_window :: call\n",
                    "        self.settings_ok = false\n",
                    "        if current.options.cursor.position.0 == 12:\n",
                    "            if current.options.cursor.position.1 == 34:\n",
                    "                if text.enabled:\n",
                    "                    if text.composition_area.active:\n",
                    "                        if text.composition_area.position.0 == 9:\n",
                    "                            if text.composition_area.position.1 == 10:\n",
                    "                                if text.composition_area.size.0 == 20:\n",
                    "                                    if text.composition_area.size.1 == 21:\n",
                    "                                        self.settings_ok = true\n",
                    "        arcana_desktop.app.set_control_flow :: cx, (arcana_desktop.types.ControlFlow.Wait :: :: call) :: call\n",
                    "        finish_if_ready :: self, cx :: call\n",
                    "\n",
                    "    fn suspended(edit self: Demo, edit cx: arcana_desktop.types.AppContext):\n",
                    "        return\n",
                    "\n",
                    "    fn window_event(edit self: Demo, edit cx: arcana_desktop.types.AppContext, read target: arcana_desktop.types.TargetedEvent) -> arcana_desktop.types.ControlFlow:\n",
                    "        return match target.event:\n",
                    "            arcana_desktop.types.WindowEvent.TextInput(ev) => on_text :: self, cx, ev :: call\n",
                    "            arcana_desktop.types.WindowEvent.TextCompositionStarted(_) => on_comp_started :: self, cx :: call\n",
                    "            arcana_desktop.types.WindowEvent.TextCompositionCancelled(_) => on_comp_cancelled :: self, cx :: call\n",
                    "            _ => cx.control.control_flow\n",
                    "\n",
                    "    fn device_event(edit self: Demo, edit cx: arcana_desktop.types.AppContext, read event: arcana_desktop.types.DeviceEvent) -> arcana_desktop.types.ControlFlow:\n",
                    "        return cx.control.control_flow\n",
                    "\n",
                    "    fn about_to_wait(edit self: Demo, edit cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.ControlFlow:\n",
                    "        finish_if_ready :: self, cx :: call\n",
                    "        return cx.control.control_flow\n",
                    "\n",
                    "    fn wake(edit self: Demo, edit cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.ControlFlow:\n",
                    "        return cx.control.control_flow\n",
                    "\n",
                    "    fn exiting(edit self: Demo, edit cx: arcana_desktop.types.AppContext):\n",
                    "        return\n",
                    "\n",
                    "fn on_text(edit self: Demo, edit cx: arcana_desktop.types.AppContext, read ev: arcana_desktop.types.TextInputEvent) -> arcana_desktop.types.ControlFlow:\n",
                    "    if ev.text == \"x\":\n",
                    "        self.saw_text = true\n",
                    "    finish_if_ready :: self, cx :: call\n",
                    "    return arcana_desktop.types.ControlFlow.Wait :: :: call\n",
                    "\n",
                    "fn on_comp_started(edit self: Demo, edit cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.ControlFlow:\n",
                    "    self.saw_comp_started = true\n",
                    "    finish_if_ready :: self, cx :: call\n",
                    "    return arcana_desktop.types.ControlFlow.Wait :: :: call\n",
                    "\n",
                    "fn on_comp_cancelled(edit self: Demo, edit cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.ControlFlow:\n",
                    "    self.saw_comp_cancelled = true\n",
                    "    finish_if_ready :: self, cx :: call\n",
                    "    return arcana_desktop.types.ControlFlow.Wait :: :: call\n",
                    "\n",
                    "fn main() -> Int:\n",
                    "    let mut cfg = arcana_desktop.app.default_app_config :: :: call\n",
                    "    cfg.window.title = {title:?}\n",
                    "    cfg.window.options.cursor.position = (12, 34)\n",
                    "    let mut app = default_demo :: :: call\n",
                    "    return arcana_desktop.app.run :: app, cfg :: call\n",
                ),
                title = title,
            ),
        );
        write_file(&dir.join("src/types.arc"), "// types\n");

        let bundle = package_workspace(dir.clone(), BuildTarget::windows_exe(), None, None)
            .expect("package should succeed");
        let exe_path = bundle.bundle_dir.join(&bundle.root_artifact);
        let mut child = Command::new(&exe_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("staged desktop text-input IME bundle should launch");
        let child_pid = child.id();
        let hwnd = match wait_for_process_window(child_pid, Duration::from_secs(5)) {
            Some((hwnd, _window_title)) => hwnd,
            None => {
                let status = wait_for_child_exit(&mut child, Duration::from_secs(1));
                if status.is_none() {
                    let _ = child.kill();
                }
                let output = child
                    .wait_with_output()
                    .expect("desktop IME child output should collect");
                panic!(
                    "desktop IME test window should open; status={:?}, stdout={}, stderr={}",
                    status.and_then(|value| value.code()),
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        };
        thread::sleep(Duration::from_millis(100));
        if let Err(err) = drive_native_text_input_and_ime(hwnd) {
            let _ = child.kill();
            let output = child
                .wait_with_output()
                .expect("desktop IME child output should collect after drive failure");
            panic!(
                "desktop IME input should drive: {err}; stdout={}, stderr={}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        let status = wait_for_child_exit(&mut child, Duration::from_secs(10));
        if status.is_none() {
            let _ = child.kill();
        }
        let output = child
            .wait_with_output()
            .expect("desktop IME child output should collect");
        assert_eq!(
            status
                .map(|value| value.code())
                .unwrap_or_else(|| output.status.code()),
            Some(0)
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .collect::<Vec<_>>(),
            vec!["1"]
        );
        assert!(
            output.stderr.is_empty(),
            "desktop IME bundle stderr should stay empty: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    #[test]
    fn package_workspace_runs_large_arcana_desktop_windows_bundle_with_runtime_dll() {
        let workspace_dir = desktop_proof_workspace_copy("windows_large_arcana_desktop_workspace");
        let exe_out_dir = temp_dir("windows_large_arcana_desktop_exe_bundle");
        let exe_bundle = package_workspace(
            workspace_dir.clone(),
            BuildTarget::windows_exe(),
            Some("app".to_string()),
            Some(exe_out_dir.clone()),
        )
        .expect("large desktop exe package should succeed");
        let exe_path = exe_bundle.bundle_dir.join(&exe_bundle.root_artifact);
        let runtime_dll_path = exe_bundle.bundle_dir.join("arcwin.dll");
        assert!(
            runtime_dll_path.is_file(),
            "expected staged desktop runtime DLL at {}",
            runtime_dll_path.display()
        );
        let rust_std_dll = fs::read_dir(&exe_bundle.bundle_dir)
            .expect("bundle dir should be readable")
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .find(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("std-") && name.ends_with(".dll"))
            });
        assert!(
            rust_std_dll.is_none(),
            "unexpected staged Rust std runtime DLL beside {}",
            runtime_dll_path.display()
        );
        let output = Command::new(&exe_path)
            .current_dir(&exe_bundle.bundle_dir)
            .env("ARCANA_NATIVE_PRODUCT_TEMP_PROBES", "1")
            .arg("--smoke")
            .output()
            .expect("large desktop exe bundle should launch");
        assert_eq!(output.status.code(), Some(0));
        assert_eq!(
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .collect::<Vec<_>>(),
            vec!["controls=36", "pages=7", "smoke_score=767"]
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("child_runtime_provider_entrypoint"),
            "expected probe-confirmed child runtime provider handoff, stderr was: {stderr}"
        );
        let _ = fs::remove_dir_all(&workspace_dir);
        let _ = fs::remove_dir_all(&exe_out_dir);
    }

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    #[test]
    fn package_workspace_closes_arcana_desktop_showcase_from_window_close_button() {
        let workspace_dir =
            desktop_proof_workspace_copy("windows_large_arcana_desktop_close_workspace");
        let exe_out_dir = temp_dir("windows_large_arcana_desktop_close_bundle");
        let exe_bundle = package_workspace(
            workspace_dir.clone(),
            BuildTarget::windows_exe(),
            Some("app".to_string()),
            Some(exe_out_dir.clone()),
        )
        .expect("large desktop exe package should succeed");
        let exe_path = exe_bundle.bundle_dir.join(&exe_bundle.root_artifact);
        let mut child = TestChild::new(
            Command::new(&exe_path)
                .current_dir(&exe_bundle.bundle_dir)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("large desktop exe bundle should launch"),
        );
        let (hwnd, _title) = wait_for_process_window(child.id(), Duration::from_secs(20))
            .expect("desktop showcase window should appear");
        unsafe {
            SendMessageW(hwnd, WM_CLOSE, 0, 0);
        }
        let status = wait_for_child_exit(&mut child, Duration::from_secs(20))
            .expect("desktop showcase should exit after WM_CLOSE");
        assert_eq!(status.code(), Some(0));
        let _ = fs::remove_dir_all(&workspace_dir);
        let _ = fs::remove_dir_all(&exe_out_dir);
    }

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    #[test]
    fn package_workspace_drives_arcana_desktop_showcase_next_page_from_mouse_click() {
        let workspace_dir =
            desktop_proof_workspace_copy("windows_large_arcana_desktop_click_workspace");
        let exe_out_dir = temp_dir("windows_large_arcana_desktop_click_bundle");
        let exe_bundle = package_workspace(
            workspace_dir.clone(),
            BuildTarget::windows_exe(),
            Some("app".to_string()),
            Some(exe_out_dir.clone()),
        )
        .expect("large desktop exe package should succeed");
        let exe_path = exe_bundle.bundle_dir.join(&exe_bundle.root_artifact);
        let mut child = TestChild::new(
            Command::new(&exe_path)
                .current_dir(&exe_bundle.bundle_dir)
                .arg("--ui-smoke")
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .expect("large desktop exe bundle should launch"),
        );
        let (hwnd, _title) = wait_for_process_window(child.id(), Duration::from_secs(20))
            .expect("desktop showcase window should appear");
        wait_for_window_title_contains(hwnd, "Overview", Duration::from_secs(10))
            .expect("desktop showcase should publish the overview title before input");
        let (x, y) = desktop_showcase_button_center(1);
        send_left_click(hwnd, x, y);
        thread::sleep(Duration::from_millis(500));
        unsafe {
            SendMessageW(hwnd, WM_CLOSE, 0, 0);
        }
        let status = wait_for_child_exit(&mut child, Duration::from_secs(20))
            .expect("desktop showcase should exit after WM_CLOSE");
        assert_eq!(status.code(), Some(0));
        let mut stdout = String::new();
        child
            .stdout
            .take()
            .expect("stdout should be captured")
            .read_to_string(&mut stdout)
            .expect("stdout should read");
        assert!(
            stdout.lines().any(|line| line == "page=Window"),
            "clicking next page should print `page=Window`, got `{stdout}`"
        );
        let _ = fs::remove_dir_all(&workspace_dir);
        let _ = fs::remove_dir_all(&exe_out_dir);
    }

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    #[test]
    fn package_workspace_clicking_second_window_does_not_crash_showcase() {
        let workspace_dir =
            desktop_proof_workspace_copy("windows_large_arcana_desktop_second_window_workspace");
        let exe_out_dir = temp_dir("windows_large_arcana_desktop_second_window_bundle");
        let exe_bundle = package_workspace(
            workspace_dir.clone(),
            BuildTarget::windows_exe(),
            Some("app".to_string()),
            Some(exe_out_dir.clone()),
        )
        .expect("large desktop exe package should succeed");
        let exe_path = exe_bundle.bundle_dir.join(&exe_bundle.root_artifact);
        let mut child = TestChild::new(
            Command::new(&exe_path)
                .current_dir(&exe_bundle.bundle_dir)
                .arg("--exercise-second-window")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("large desktop exe bundle should launch"),
        );
        let (main_hwnd, _title) =
            wait_for_named_process_window(child.id(), "Overview", Duration::from_secs(30))
                .expect("desktop showcase main window should appear");
        let (second_hwnd, _title) = if let Some(found) = wait_for_additional_named_process_window(
            child.id(),
            main_hwnd,
            "Second Window",
            Duration::from_secs(30),
        ) {
            found
        } else {
            let _ = wait_for_child_exit(&mut child, Duration::from_secs(20));
            let mut stdout = String::new();
            child
                .stdout
                .take()
                .expect("stdout should be captured")
                .read_to_string(&mut stdout)
                .expect("stdout should read");
            let mut stderr = String::new();
            child
                .stderr
                .take()
                .expect("stderr should be captured")
                .read_to_string(&mut stderr)
                .expect("stderr should read");
            let windows = process_windows(child.id())
                .into_iter()
                .map(|(_, title)| title)
                .collect::<Vec<_>>();
            panic!(
                "secondary showcase window should appear; stdout was `{stdout}`; stderr was `{stderr}`; windows={windows:?}"
            );
        };
        thread::sleep(Duration::from_millis(200));
        send_left_click(second_hwnd, 80, 80);
        thread::sleep(Duration::from_millis(500));
        assert!(
            child
                .try_wait()
                .expect("desktop showcase child state should be observable")
                .is_none(),
            "desktop showcase should stay alive after second-window click"
        );
        unsafe {
            SendMessageW(main_hwnd, WM_CLOSE, 0, 0);
        }
        thread::sleep(Duration::from_millis(500));
        assert!(
            child
                .try_wait()
                .expect("desktop showcase child state should still be observable after main close")
                .is_none(),
            "desktop showcase should stay alive while the secondary window remains open"
        );
        unsafe {
            SendMessageW(second_hwnd, WM_CLOSE, 0, 0);
        }
        let status = wait_for_child_exit(&mut child, Duration::from_secs(20))
            .expect("desktop showcase should exit after all windows close");
        let mut stdout = String::new();
        child
            .stdout
            .take()
            .expect("stdout should be captured")
            .read_to_string(&mut stdout)
            .expect("stdout should read");
        let mut stderr = String::new();
        child
            .stderr
            .take()
            .expect("stderr should be captured")
            .read_to_string(&mut stderr)
            .expect("stderr should read");
        assert_eq!(status.code(), Some(0));
        assert!(
            stdout
                .lines()
                .any(|line| line.starts_with("second_window=")),
            "exercise mode should print the opened second window id, got `{stdout}`"
        );
        assert!(
            stdout
                .lines()
                .any(|line| line.starts_with("second_window=click:")),
            "second-window click should be delivered through the showcase, got `{stdout}`"
        );
        assert!(
            stderr.is_empty(),
            "showcase second-window exercise should not write stderr: `{stderr}`"
        );
        let _ = fs::remove_dir_all(&workspace_dir);
        let _ = fs::remove_dir_all(&exe_out_dir);
    }

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    #[test]
    fn package_workspace_main_button_closes_showcase_second_window() {
        let workspace_dir =
            desktop_proof_workspace_copy("windows_large_arcana_desktop_close_second_workspace");
        let exe_out_dir = temp_dir("windows_large_arcana_desktop_close_second_bundle");
        let exe_bundle = package_workspace(
            workspace_dir.clone(),
            BuildTarget::windows_exe(),
            Some("app".to_string()),
            Some(exe_out_dir.clone()),
        )
        .expect("large desktop exe package should succeed");
        let exe_path = exe_bundle.bundle_dir.join(&exe_bundle.root_artifact);
        let mut child = TestChild::new(
            Command::new(&exe_path)
                .current_dir(&exe_bundle.bundle_dir)
                .arg("--exercise-second-window")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("large desktop exe bundle should launch"),
        );
        let (main_hwnd, _title) =
            wait_for_named_process_window(child.id(), "Overview", Duration::from_secs(30))
                .expect("desktop showcase main window should appear");
        let (second_hwnd, _title) = wait_for_additional_named_process_window(
            child.id(),
            main_hwnd,
            "Second Window",
            Duration::from_secs(30),
        )
        .expect("secondary showcase window should appear");
        let (x, y) = desktop_showcase_button_center(35);
        let windows = click_until_window_gone(
            child.id(),
            main_hwnd,
            x,
            y,
            second_hwnd,
            Duration::from_secs(20),
        );
        assert!(
            windows.iter().all(|(hwnd, _)| *hwnd != second_hwnd),
            "main-button second-window close should remove the secondary window, got windows={windows:?}"
        );
        assert!(
            child
                .try_wait()
                .expect("desktop showcase child state should be observable")
                .is_none(),
            "desktop showcase should stay alive after closing the secondary window from the main window"
        );
        unsafe {
            SendMessageW(main_hwnd, WM_CLOSE, 0, 0);
        }
        let status = wait_for_child_exit(&mut child, Duration::from_secs(20))
            .expect("desktop showcase should exit after main window closes");
        let mut stdout = String::new();
        child
            .stdout
            .take()
            .expect("stdout should be captured")
            .read_to_string(&mut stdout)
            .expect("stdout should read");
        let mut stderr = String::new();
        child
            .stderr
            .take()
            .expect("stderr should be captured")
            .read_to_string(&mut stderr)
            .expect("stderr should read");
        assert_eq!(status.code(), Some(0));
        assert!(
            stdout
                .lines()
                .any(|line| line.starts_with("second_window=open:")),
            "exercise mode should print the opened second window id, got `{stdout}`"
        );
        assert!(
            stderr.is_empty(),
            "showcase close-second-window path should not write stderr: `{stderr}`"
        );
        let _ = fs::remove_dir_all(&workspace_dir);
        let _ = fs::remove_dir_all(&exe_out_dir);
    }

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
    #[test]
    fn package_workspace_main_button_reopens_showcase_second_window_after_close() {
        let workspace_dir =
            desktop_proof_workspace_copy("windows_large_arcana_desktop_reopen_second_workspace");
        let exe_out_dir = temp_dir("windows_large_arcana_desktop_reopen_second_bundle");
        let exe_bundle = package_workspace(
            workspace_dir.clone(),
            BuildTarget::windows_exe(),
            Some("app".to_string()),
            Some(exe_out_dir.clone()),
        )
        .expect("large desktop exe package should succeed");
        let exe_path = exe_bundle.bundle_dir.join(&exe_bundle.root_artifact);
        let mut child = TestChild::new(
            Command::new(&exe_path)
                .current_dir(&exe_bundle.bundle_dir)
                .arg("--exercise-second-window")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("large desktop exe bundle should launch"),
        );
        let (main_hwnd, _title) =
            wait_for_named_process_window(child.id(), "Overview", Duration::from_secs(30))
                .expect("desktop showcase main window should appear");
        let (first_second_hwnd, _title) = wait_for_additional_named_process_window(
            child.id(),
            main_hwnd,
            "Second Window",
            Duration::from_secs(30),
        )
        .expect("secondary showcase window should appear");

        let (close_x, close_y) = desktop_showcase_button_center(35);
        let windows = click_until_window_gone(
            child.id(),
            main_hwnd,
            close_x,
            close_y,
            first_second_hwnd,
            Duration::from_secs(20),
        );
        assert!(
            windows.iter().all(|(hwnd, _)| *hwnd != first_second_hwnd),
            "main-button close should remove the original secondary window before reopen, got windows={windows:?}"
        );

        let (open_x, open_y) = desktop_showcase_button_center(22);
        send_left_click(main_hwnd, open_x, open_y);
        let (reopened_hwnd, _title) = wait_for_additional_named_process_window(
            child.id(),
            main_hwnd,
            "Second Window",
            Duration::from_secs(30),
        )
        .expect("secondary showcase window should reopen after close");

        assert!(
            child
                .try_wait()
                .expect("desktop showcase child state should be observable")
                .is_none(),
            "desktop showcase should stay alive after reopening the secondary window"
        );

        unsafe {
            SendMessageW(reopened_hwnd, WM_CLOSE, 0, 0);
        }
        unsafe {
            SendMessageW(main_hwnd, WM_CLOSE, 0, 0);
        }
        let status = wait_for_child_exit(&mut child, Duration::from_secs(20)).expect(
            "desktop showcase should exit after closing reopened secondary and main windows",
        );
        let mut stdout = String::new();
        child
            .stdout
            .take()
            .expect("stdout should be captured")
            .read_to_string(&mut stdout)
            .expect("stdout should read");
        let mut stderr = String::new();
        child
            .stderr
            .take()
            .expect("stderr should be captured")
            .read_to_string(&mut stderr)
            .expect("stderr should read");
        assert_eq!(status.code(), Some(0));
        assert!(
            stdout
                .lines()
                .filter(|line| line.starts_with("second_window=open:"))
                .count()
                >= 2,
            "exercise mode should report the initial and reopened second-window opens, got `{stdout}`"
        );
        assert!(
            stderr.is_empty(),
            "showcase reopen path should not write stderr: `{stderr}`"
        );
        let _ = fs::remove_dir_all(&workspace_dir);
        let _ = fs::remove_dir_all(&exe_out_dir);
    }

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
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

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
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

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
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

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
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

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
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

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
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

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
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

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
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

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
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

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
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

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
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

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
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

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
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

    #[cfg(all(windows, feature = "legacy-native-host-tests"))]
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
