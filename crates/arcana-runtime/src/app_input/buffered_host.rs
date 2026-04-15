use super::*;

#[derive(Debug)]
pub(crate) struct BufferedHostStream {
    path: String,
    file: fs::File,
    readable: bool,
    writable: bool,
}

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

    fn current_working_dir(&self) -> Result<PathBuf, String> {
        if !self.cwd.is_empty() {
            return Ok(normalize_lexical_path(Path::new(&self.cwd)));
        }
        std::env::current_dir()
            .map(|path| normalize_lexical_path(&path))
            .map_err(|err| format!("failed to resolve current directory: {err}"))
    }

    fn sandbox_root_path(&self) -> Result<Option<PathBuf>, String> {
        if self.sandbox_root.is_empty() {
            return Ok(None);
        }
        Ok(Some(normalize_lexical_path(Path::new(&self.sandbox_root))))
    }

    fn sandbox_checked_real_path(&self, path: &Path) -> Result<PathBuf, String> {
        let mut current = Some(path);
        while let Some(candidate) = current {
            if candidate.exists() {
                let real = fs::canonicalize(candidate).map_err(|err| {
                    format!(
                        "failed to canonicalize `{}`: {err}",
                        runtime_path_string(candidate)
                    )
                })?;
                let suffix = path.strip_prefix(candidate).map_err(|_| {
                    format!(
                        "failed to make `{}` relative to checked ancestor `{}`",
                        runtime_path_string(path),
                        runtime_path_string(candidate)
                    )
                })?;
                return Ok(normalize_lexical_path(&real.join(suffix)));
            }
            current = candidate.parent();
        }
        Ok(normalize_lexical_path(path))
    }

    pub fn resolve_fs_path(&self, path: &str) -> Result<PathBuf, String> {
        let requested = PathBuf::from(path);
        let candidate = if requested.is_absolute() {
            normalize_lexical_path(&requested)
        } else {
            normalize_lexical_path(&self.current_working_dir()?.join(requested))
        };
        if let Some(root) = self.sandbox_root_path()? {
            if !candidate.starts_with(&root) {
                return Err(format!(
                    "path `{}` escapes sandbox root `{}`",
                    runtime_path_string(&candidate),
                    runtime_path_string(&root)
                ));
            }
            let real_root = self.sandbox_checked_real_path(&root)?;
            let real_candidate = self.sandbox_checked_real_path(&candidate)?;
            if !real_candidate.starts_with(&real_root) {
                return Err(format!(
                    "path `{}` escapes sandbox root `{}` via real path `{}`",
                    runtime_path_string(&candidate),
                    runtime_path_string(&root),
                    runtime_path_string(&real_candidate)
                ));
            }
        }
        Ok(candidate)
    }

    pub(crate) fn path_canonicalize(&self, path: &str) -> Result<String, String> {
        let resolved = self.resolve_fs_path(path)?;
        Ok(runtime_path_string(
            &self.sandbox_checked_real_path(&resolved)?,
        ))
    }

    pub(crate) fn fs_read_text(&self, path: &str) -> Result<String, String> {
        let resolved = self.resolve_fs_path(path)?;
        fs::read_to_string(&resolved)
            .map_err(|err| format!("failed to read `{}`: {err}", runtime_path_string(&resolved)))
    }

    pub(crate) fn fs_write_text(&self, path: &str, text: &str) -> Result<(), String> {
        let resolved = self.resolve_fs_path(path)?;
        if let Some(parent) = resolved.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                format!(
                    "failed to create parent directories for `{}`: {err}",
                    runtime_path_string(&resolved)
                )
            })?;
        }
        fs::write(&resolved, text).map_err(|err| {
            format!(
                "failed to write `{}`: {err}",
                runtime_path_string(&resolved)
            )
        })
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
        self.current_working_dir()
    }

    fn runtime_resolve_fs_path(&self, path: &str) -> Result<PathBuf, String> {
        self.resolve_fs_path(path)
    }

    fn runtime_path_canonicalize(&self, path: &str) -> Result<String, String> {
        self.path_canonicalize(path)
    }

    fn runtime_fs_stream_open_read(&mut self, path: &str) -> Result<u64, String> {
        let resolved = self.resolve_fs_path(path)?;
        let file = fs::File::open(&resolved).map_err(|err| {
            format!(
                "failed to open `{}` for reading: {err}",
                runtime_path_string(&resolved)
            )
        })?;
        let handle = self.next_stream_handle();
        self.streams.insert(
            handle,
            BufferedHostStream {
                path: runtime_path_string(&resolved),
                file,
                readable: true,
                writable: false,
            },
        );
        Ok(handle)
    }

    fn runtime_fs_stream_open_write(&mut self, path: &str, append: bool) -> Result<u64, String> {
        let resolved = self.resolve_fs_path(path)?;
        if let Some(parent) = resolved.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                format!("failed to prepare `{}`: {err}", runtime_path_string(parent))
            })?;
        }
        let mut options = fs::OpenOptions::new();
        options.create(true).write(true);
        if append {
            options.append(true);
        } else {
            options.truncate(true);
        }
        let file = options.open(&resolved).map_err(|err| {
            format!(
                "failed to open `{}` for writing: {err}",
                runtime_path_string(&resolved)
            )
        })?;
        let handle = self.next_stream_handle();
        self.streams.insert(
            handle,
            BufferedHostStream {
                path: runtime_path_string(&resolved),
                file,
                readable: false,
                writable: true,
            },
        );
        Ok(handle)
    }

    fn runtime_fs_stream_read(&mut self, handle: u64, max_bytes: usize) -> Result<Vec<u8>, String> {
        let stream = self.stream_mut(handle)?;
        if !stream.readable {
            return Err(format!(
                "FileStream `{}` is not opened for reading",
                stream.path
            ));
        }
        use std::io::Read;
        let mut buffer = vec![0u8; max_bytes];
        let read = stream
            .file
            .read(&mut buffer)
            .map_err(|err| format!("failed to read from FileStream `{}`: {err}", stream.path))?;
        buffer.truncate(read);
        Ok(buffer)
    }

    fn runtime_fs_stream_write(&mut self, handle: u64, bytes: &[u8]) -> Result<usize, String> {
        let stream = self.stream_mut(handle)?;
        if !stream.writable {
            return Err(format!(
                "FileStream `{}` is not opened for writing",
                stream.path
            ));
        }
        use std::io::Write;
        stream
            .file
            .write_all(bytes)
            .map_err(|err| format!("failed to write to FileStream `{}`: {err}", stream.path))?;
        Ok(bytes.len())
    }

    fn runtime_fs_stream_eof(&mut self, handle: u64) -> Result<bool, String> {
        let stream = self.stream_mut(handle)?;
        if !stream.readable {
            return Err(format!(
                "FileStream `{}` is not opened for reading",
                stream.path
            ));
        }
        use std::io::Seek;
        let cursor = stream
            .file
            .stream_position()
            .map_err(|err| format!("failed to inspect FileStream `{}`: {err}", stream.path))?;
        let len = stream
            .file
            .metadata()
            .map_err(|err| format!("failed to stat FileStream `{}`: {err}", stream.path))?
            .len();
        Ok(cursor >= len)
    }

    fn runtime_fs_stream_close(&mut self, handle: u64) -> Result<(), String> {
        self.streams
            .remove(&handle)
            .map(|_| ())
            .ok_or_else(|| format!("invalid FileStream handle `{handle}`"))
    }
}
