use std::collections::BTreeMap;
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::{
    ARCANA_NATIVE_BUNDLE_DIR_ENV, HostCoreFsPolicy, HostCoreStreamState, RuntimeCoreHost,
    RuntimeProcessCapture, normalize_lexical_path,
};

type ProcessRuntimeHostStream = HostCoreStreamState;

#[derive(Clone, Debug)]
pub struct ProcessRuntimeHostConfig {
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub cwd: PathBuf,
    pub sandbox_root: PathBuf,
    pub allow_process: bool,
}

impl ProcessRuntimeHostConfig {
    pub fn from_current_process() -> Result<Self, String> {
        let cwd = std::env::current_dir()
            .map(|path| normalize_lexical_path(&path))
            .map_err(|err| format!("failed to resolve current directory: {err}"))?;
        let sandbox_root = std::env::var(ARCANA_NATIVE_BUNDLE_DIR_ENV)
            .map(PathBuf::from)
            .map(|path| normalize_lexical_path(&path))
            .unwrap_or_else(|_| cwd.clone());
        let mut args = Vec::new();
        let mut allow_process = false;
        for arg in std::env::args().skip(1) {
            if arg == "--allow-process" {
                allow_process = true;
                continue;
            }
            args.push(arg);
        }
        Ok(Self {
            args,
            env: std::env::vars().collect(),
            cwd,
            sandbox_root,
            allow_process,
        })
    }
}

pub(crate) struct ProcessRuntimeHost {
    start: Instant,
    config: ProcessRuntimeHostConfig,
    fs_policy: HostCoreFsPolicy,
    next_stream_handle: u64,
    streams: BTreeMap<u64, ProcessRuntimeHostStream>,
}

impl ProcessRuntimeHost {
    pub(crate) fn from_config(config: ProcessRuntimeHostConfig) -> Self {
        Self {
            start: Instant::now(),
            fs_policy: HostCoreFsPolicy::new(config.cwd.clone(), Some(config.sandbox_root.clone())),
            config,
            next_stream_handle: 1,
            streams: BTreeMap::new(),
        }
    }

    fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    fn next_stream_handle(&mut self) -> u64 {
        let handle = self.next_stream_handle.max(1);
        self.next_stream_handle = handle + 1;
        handle
    }

    fn stream_mut(&mut self, handle: u64) -> Result<&mut ProcessRuntimeHostStream, String> {
        self.streams
            .get_mut(&handle)
            .ok_or_else(|| format!("invalid FileStream handle `{handle}`"))
    }
}

impl RuntimeCoreHost for ProcessRuntimeHost {
    fn print(&mut self, text: &str) -> Result<(), String> {
        let mut stdout = std::io::stdout().lock();
        stdout
            .write_all(text.as_bytes())
            .map_err(|err| format!("failed to write stdout: {err}"))
    }

    fn eprint(&mut self, text: &str) -> Result<(), String> {
        let mut stderr = std::io::stderr().lock();
        stderr
            .write_all(text.as_bytes())
            .map_err(|err| format!("failed to write stderr: {err}"))
    }

    fn flush_stdout(&mut self) -> Result<(), String> {
        std::io::stdout()
            .lock()
            .flush()
            .map_err(|err| format!("failed to flush stdout: {err}"))
    }

    fn flush_stderr(&mut self) -> Result<(), String> {
        std::io::stderr()
            .lock()
            .flush()
            .map_err(|err| format!("failed to flush stderr: {err}"))
    }

    fn stdin_read_line(&mut self) -> Result<String, String> {
        let mut line = String::new();
        std::io::stdin()
            .lock()
            .read_line(&mut line)
            .map_err(|err| format!("failed to read stdin: {err}"))?;
        while line.ends_with(['\n', '\r']) {
            line.pop();
        }
        Ok(line)
    }

    fn monotonic_now_ms(&mut self) -> Result<i64, String> {
        i64::try_from(self.elapsed().as_millis())
            .map_err(|_| "monotonic millisecond clock overflowed i64".to_string())
    }

    fn monotonic_now_ns(&mut self) -> Result<i64, String> {
        i64::try_from(self.elapsed().as_nanos())
            .map_err(|_| "monotonic nanosecond clock overflowed i64".to_string())
    }

    fn sleep_ms(&mut self, ms: i64) -> Result<(), String> {
        if ms < 0 {
            return Err("sleep_ms expects a non-negative duration".to_string());
        }
        std::thread::sleep(Duration::from_millis(ms as u64));
        Ok(())
    }

    fn allows_process_execution(&self) -> bool {
        self.config.allow_process
    }

    fn runtime_arg_count(&self) -> Result<i64, String> {
        Ok(self.config.args.len() as i64)
    }

    fn runtime_arg_get(&self, index: i64) -> Result<String, String> {
        if index < 0 {
            return Err("arg_get index must be non-negative".to_string());
        }
        Ok(self
            .config
            .args
            .get(index as usize)
            .cloned()
            .unwrap_or_default())
    }

    fn runtime_env_has(&self, name: &str) -> Result<bool, String> {
        Ok(self.config.env.contains_key(name))
    }

    fn runtime_env_get(&self, name: &str) -> Result<String, String> {
        Ok(self.config.env.get(name).cloned().unwrap_or_default())
    }

    fn runtime_current_working_dir(&self) -> Result<PathBuf, String> {
        Ok(self.fs_policy.current_working_dir())
    }

    fn runtime_resolve_fs_path(&self, path: &str) -> Result<PathBuf, String> {
        self.fs_policy.resolve_fs_path(path)
    }

    fn runtime_path_canonicalize(&self, path: &str) -> Result<String, String> {
        self.fs_policy.path_canonicalize(path)
    }

    fn runtime_fs_stream_open_read(&mut self, path: &str) -> Result<u64, String> {
        let handle = self.next_stream_handle();
        let stream = ProcessRuntimeHostStream::open_read(&self.fs_policy, path)?;
        self.streams.insert(handle, stream);
        Ok(handle)
    }

    fn runtime_fs_stream_open_write(&mut self, path: &str, append: bool) -> Result<u64, String> {
        let handle = self.next_stream_handle();
        let stream = ProcessRuntimeHostStream::open_write(&self.fs_policy, path, append)?;
        self.streams.insert(handle, stream);
        Ok(handle)
    }

    fn runtime_fs_stream_read(&mut self, handle: u64, max_bytes: usize) -> Result<Vec<u8>, String> {
        self.stream_mut(handle)?.read(max_bytes)
    }

    fn runtime_fs_stream_write(&mut self, handle: u64, bytes: &[u8]) -> Result<usize, String> {
        self.stream_mut(handle)?.write(bytes)
    }

    fn runtime_fs_stream_eof(&mut self, handle: u64) -> Result<bool, String> {
        self.stream_mut(handle)?.eof()
    }

    fn runtime_fs_stream_close(&mut self, handle: u64) -> Result<(), String> {
        self.streams
            .remove(&handle)
            .map(|_| ())
            .ok_or_else(|| format!("invalid FileStream handle `{handle}`"))
    }

    fn runtime_process_exec_status(
        &mut self,
        program: &str,
        args: &[String],
    ) -> Result<i64, String> {
        self.fs_policy
            .execute_process_status(self.config.allow_process, program, args)
    }

    fn runtime_process_exec_capture(
        &mut self,
        program: &str,
        args: &[String],
    ) -> Result<RuntimeProcessCapture, String> {
        self.fs_policy
            .execute_process_capture(self.config.allow_process, program, args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir(label: &str) -> PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("arcana-process-host-{label}-{stamp}"));
        fs::create_dir_all(&dir).expect("temp dir should create");
        dir
    }

    #[test]
    fn configured_host_filters_allow_process_flag_from_public_args() {
        let dir = temp_dir("args");
        let host = ProcessRuntimeHost::from_config(ProcessRuntimeHostConfig {
            args: vec!["visible".to_string()],
            env: BTreeMap::new(),
            cwd: dir.clone(),
            sandbox_root: dir.clone(),
            allow_process: true,
        });
        assert!(host.allows_process_execution());
        assert_eq!(host.runtime_arg_count().expect("arg count should work"), 1);
        assert_eq!(
            host.runtime_arg_get(0).expect("arg get should work"),
            "visible"
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn configured_host_rejects_sandbox_escape_paths() {
        let dir = temp_dir("sandbox");
        let host = ProcessRuntimeHost::from_config(ProcessRuntimeHostConfig {
            args: Vec::new(),
            env: BTreeMap::new(),
            cwd: dir.clone(),
            sandbox_root: dir.clone(),
            allow_process: false,
        });
        let outside = dir
            .parent()
            .expect("temp dir should have parent")
            .join("outside.txt");
        let err = host
            .runtime_resolve_fs_path(outside.to_string_lossy().as_ref())
            .expect_err("absolute path outside root should fail");
        assert!(err.contains("escapes sandbox root"), "{err}");

        let err = host
            .runtime_resolve_fs_path("../escape.txt")
            .expect_err("parent escape should fail");
        assert!(err.contains("escapes sandbox root"), "{err}");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn configured_host_denies_process_execution_without_flag() {
        let dir = temp_dir("process");
        let mut host = ProcessRuntimeHost::from_config(ProcessRuntimeHostConfig {
            args: Vec::new(),
            env: BTreeMap::new(),
            cwd: dir.clone(),
            sandbox_root: dir.clone(),
            allow_process: false,
        });
        let err = host
            .runtime_process_exec_status("child.exe", &[])
            .expect_err("process execution should be denied");
        assert!(err.contains("process execution is disabled"), "{err}");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn configured_host_reads_env_from_explicit_config() {
        let dir = temp_dir("env");
        let mut env = BTreeMap::new();
        env.insert("ARCANA_SAMPLE".to_string(), "value".to_string());
        let host = ProcessRuntimeHost::from_config(ProcessRuntimeHostConfig {
            args: Vec::new(),
            env,
            cwd: dir.clone(),
            sandbox_root: dir.clone(),
            allow_process: false,
        });
        assert!(
            host.runtime_env_has("ARCANA_SAMPLE")
                .expect("env has should work")
        );
        assert_eq!(
            host.runtime_env_get("ARCANA_SAMPLE")
                .expect("env get should work"),
            "value"
        );
        let _ = fs::remove_dir_all(dir);
    }
}
