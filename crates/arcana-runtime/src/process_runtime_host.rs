use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, Read, Seek, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::RuntimeCoreHost;

#[derive(Debug)]
struct ProcessRuntimeHostStream {
    path: String,
    file: File,
    readable: bool,
    writable: bool,
}

pub(crate) struct ProcessRuntimeHost {
    start: Instant,
    next_stream_handle: u64,
    streams: BTreeMap<u64, ProcessRuntimeHostStream>,
}

impl ProcessRuntimeHost {
    pub(crate) fn current_process() -> Self {
        Self {
            start: Instant::now(),
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

    fn stream_path(&self, path: &str) -> Result<PathBuf, String> {
        self.runtime_resolve_fs_path(path)
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

    fn runtime_fs_stream_open_read(&mut self, path: &str) -> Result<u64, String> {
        let resolved = self.stream_path(path)?;
        let file = File::open(&resolved).map_err(|err| {
            format!(
                "failed to open `{}` for reading: {err}",
                resolved.to_string_lossy()
            )
        })?;
        let handle = self.next_stream_handle();
        self.streams.insert(
            handle,
            ProcessRuntimeHostStream {
                path: resolved.to_string_lossy().into_owned(),
                file,
                readable: true,
                writable: false,
            },
        );
        Ok(handle)
    }

    fn runtime_fs_stream_open_write(&mut self, path: &str, append: bool) -> Result<u64, String> {
        let resolved = self.stream_path(path)?;
        if let Some(parent) = resolved.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                format!("failed to prepare `{}`: {err}", parent.to_string_lossy())
            })?;
        }
        let mut options = OpenOptions::new();
        options.create(true).write(true);
        if append {
            options.append(true);
        } else {
            options.truncate(true);
        }
        let file = options.open(&resolved).map_err(|err| {
            format!(
                "failed to open `{}` for writing: {err}",
                resolved.to_string_lossy()
            )
        })?;
        let handle = self.next_stream_handle();
        self.streams.insert(
            handle,
            ProcessRuntimeHostStream {
                path: resolved.to_string_lossy().into_owned(),
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
