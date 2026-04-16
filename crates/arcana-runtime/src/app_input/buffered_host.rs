use super::*;

pub(crate) type BufferedHostStream = HostCoreStreamState;

#[derive(Debug, Default)]
pub struct BufferedHost {
    pub stdout: Vec<String>,
    pub stderr: Vec<String>,
    pub stdout_flushes: usize,
    pub stderr_flushes: usize,
    pub stdin: Vec<String>,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub supported_runtime_requirements: Option<BTreeSet<String>>,
    pub allow_process: bool,
    pub cwd: String,
    pub sandbox_root: String,
    pub monotonic_now_ms: i64,
    pub monotonic_now_ns: i64,
    pub monotonic_step_ms: i64,
    pub monotonic_step_ns: i64,
    pub sleep_log_ms: Vec<i64>,
    pub(crate) next_stream_handle: u64,
    pub(crate) streams: BTreeMap<u64, BufferedHostStream>,
}

impl BufferedHost {
    pub fn current_process() -> Result<Self, String> {
        let cwd = std::env::current_dir()
            .map(|path| path.to_string_lossy().into_owned())
            .map_err(|err| format!("failed to resolve current directory: {err}"))?;
        Ok(Self {
            args: std::env::args().skip(1).collect(),
            env: std::env::vars().collect(),
            allow_process: true,
            cwd,
            ..Self::default()
        })
    }

    fn fs_policy(&self) -> Result<HostCoreFsPolicy, String> {
        let cwd = if !self.cwd.is_empty() {
            normalize_lexical_path(Path::new(&self.cwd))
        } else {
            std::env::current_dir()
                .map(|path| normalize_lexical_path(&path))
                .map_err(|err| format!("failed to resolve current directory: {err}"))?
        };
        let sandbox_root = (!self.sandbox_root.is_empty())
            .then(|| normalize_lexical_path(Path::new(&self.sandbox_root)));
        Ok(HostCoreFsPolicy::new(cwd, sandbox_root))
    }

    pub fn resolve_fs_path(&self, path: &str) -> Result<PathBuf, String> {
        self.fs_policy()?.resolve_fs_path(path)
    }

    pub(crate) fn path_canonicalize(&self, path: &str) -> Result<String, String> {
        self.fs_policy()?.path_canonicalize(path)
    }

    pub(crate) fn fs_read_text(&self, path: &str) -> Result<String, String> {
        self.fs_policy()?.read_text(path)
    }

    pub(crate) fn fs_write_text(&self, path: &str, text: &str) -> Result<(), String> {
        self.fs_policy()?.write_text(path, text)
    }

    fn next_stream_handle(&mut self) -> u64 {
        let handle = self.next_stream_handle.max(1);
        self.next_stream_handle = handle + 1;
        handle
    }

    fn stream_mut(&mut self, handle: u64) -> Result<&mut BufferedHostStream, String> {
        self.streams
            .get_mut(&handle)
            .ok_or_else(|| format!("invalid FileStream handle `{handle}`"))
    }
}

impl RuntimeCoreHost for BufferedHost {
    fn supports_runtime_requirement(&self, requirement: &str) -> bool {
        self.supported_runtime_requirements
            .as_ref()
            .is_none_or(|supported| supported.contains(requirement))
    }

    fn print(&mut self, text: &str) -> Result<(), String> {
        self.stdout.push(text.to_string());
        Ok(())
    }

    fn eprint(&mut self, text: &str) -> Result<(), String> {
        self.stderr.push(text.to_string());
        Ok(())
    }

    fn flush_stdout(&mut self) -> Result<(), String> {
        self.stdout_flushes += 1;
        Ok(())
    }

    fn flush_stderr(&mut self) -> Result<(), String> {
        self.stderr_flushes += 1;
        Ok(())
    }

    fn stdin_read_line(&mut self) -> Result<String, String> {
        if self.stdin.is_empty() {
            return Err("stdin has no queued line".to_string());
        }
        Ok(self.stdin.remove(0))
    }

    fn monotonic_now_ms(&mut self) -> Result<i64, String> {
        let now = self.monotonic_now_ms;
        self.monotonic_now_ms += self.monotonic_step_ms;
        Ok(now)
    }

    fn monotonic_now_ns(&mut self) -> Result<i64, String> {
        let now = self.monotonic_now_ns;
        self.monotonic_now_ns += self.monotonic_step_ns;
        Ok(now)
    }

    fn sleep_ms(&mut self, ms: i64) -> Result<(), String> {
        self.sleep_log_ms.push(ms);
        Ok(())
    }

    fn allows_process_execution(&self) -> bool {
        self.allow_process
    }

    fn runtime_arg_count(&self) -> Result<i64, String> {
        Ok(self.args.len() as i64)
    }

    fn runtime_arg_get(&self, index: i64) -> Result<String, String> {
        if index < 0 {
            return Err("arg_get index must be non-negative".to_string());
        }
        Ok(self.args.get(index as usize).cloned().unwrap_or_default())
    }

    fn runtime_env_has(&self, name: &str) -> Result<bool, String> {
        Ok(self.env.contains_key(name))
    }

    fn runtime_env_get(&self, name: &str) -> Result<String, String> {
        Ok(self.env.get(name).cloned().unwrap_or_default())
    }

    fn runtime_current_working_dir(&self) -> Result<PathBuf, String> {
        Ok(self.fs_policy()?.current_working_dir())
    }

    fn runtime_resolve_fs_path(&self, path: &str) -> Result<PathBuf, String> {
        self.resolve_fs_path(path)
    }

    fn runtime_path_canonicalize(&self, path: &str) -> Result<String, String> {
        self.path_canonicalize(path)
    }

    fn runtime_fs_read_text(&self, path: &str) -> Result<String, String> {
        self.fs_read_text(path)
    }

    fn runtime_fs_write_text(&self, path: &str, text: &str) -> Result<(), String> {
        self.fs_write_text(path, text)
    }

    fn runtime_fs_stream_open_read(&mut self, path: &str) -> Result<u64, String> {
        let handle = self.next_stream_handle();
        let stream = BufferedHostStream::open_read(&self.fs_policy()?, path)?;
        self.streams.insert(handle, stream);
        Ok(handle)
    }

    fn runtime_fs_stream_open_write(&mut self, path: &str, append: bool) -> Result<u64, String> {
        let handle = self.next_stream_handle();
        let stream = BufferedHostStream::open_write(&self.fs_policy()?, path, append)?;
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
        self.fs_policy()?
            .execute_process_status(self.allow_process, program, args)
    }

    fn runtime_process_exec_capture(
        &mut self,
        program: &str,
        args: &[String],
    ) -> Result<RuntimeProcessCapture, String> {
        self.fs_policy()?
            .execute_process_capture(self.allow_process, program, args)
    }
}
