use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};

use crate::{RuntimeProcessCapture, normalize_lexical_path, runtime_path_string};

#[derive(Clone, Debug)]
pub(crate) struct HostCoreFsPolicy {
    cwd: PathBuf,
    sandbox_root: Option<PathBuf>,
}

impl HostCoreFsPolicy {
    pub(crate) fn new(cwd: PathBuf, sandbox_root: Option<PathBuf>) -> Self {
        Self {
            cwd: normalize_lexical_path(&cwd),
            sandbox_root: sandbox_root.map(|path| normalize_lexical_path(&path)),
        }
    }

    pub(crate) fn current_working_dir(&self) -> PathBuf {
        self.cwd.clone()
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

    pub(crate) fn resolve_fs_path(&self, path: &str) -> Result<PathBuf, String> {
        let requested = PathBuf::from(path);
        let candidate = if requested.is_absolute() {
            normalize_lexical_path(&requested)
        } else {
            normalize_lexical_path(&self.current_working_dir().join(requested))
        };
        if let Some(root) = &self.sandbox_root {
            if !candidate.starts_with(root) {
                return Err(format!(
                    "path `{}` escapes sandbox root `{}`",
                    runtime_path_string(&candidate),
                    runtime_path_string(root)
                ));
            }
            let real_root = self.sandbox_checked_real_path(root)?;
            let real_candidate = self.sandbox_checked_real_path(&candidate)?;
            if !real_candidate.starts_with(&real_root) {
                return Err(format!(
                    "path `{}` escapes sandbox root `{}` via real path `{}`",
                    runtime_path_string(&candidate),
                    runtime_path_string(root),
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

    #[cfg(test)]
    pub(crate) fn read_text(&self, path: &str) -> Result<String, String> {
        let resolved = self.resolve_fs_path(path)?;
        fs::read_to_string(&resolved)
            .map_err(|err| format!("failed to read `{}`: {err}", runtime_path_string(&resolved)))
    }

    #[cfg(test)]
    pub(crate) fn write_text(&self, path: &str, text: &str) -> Result<(), String> {
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

    pub(crate) fn execute_process_status(
        &self,
        allow_process: bool,
        program: &str,
        args: &[String],
    ) -> Result<i64, String> {
        if !allow_process {
            return Err("process execution is disabled by the runtime host".to_string());
        }
        let resolved = self.resolve_fs_path(program)?;
        std::process::Command::new(&resolved)
            .args(args)
            .status()
            .map(|status| i64::from(status.code().unwrap_or(-1)))
            .map_err(|err| {
                format!(
                    "failed to run process `{}`: {err}",
                    runtime_path_string(&resolved)
                )
            })
    }

    pub(crate) fn execute_process_capture(
        &self,
        allow_process: bool,
        program: &str,
        args: &[String],
    ) -> Result<RuntimeProcessCapture, String> {
        if !allow_process {
            return Err("process execution is disabled by the runtime host".to_string());
        }
        let resolved = self.resolve_fs_path(program)?;
        std::process::Command::new(&resolved)
            .args(args)
            .output()
            .map(|output| RuntimeProcessCapture {
                status: i64::from(output.status.code().unwrap_or(-1)),
                stdout_utf8: std::str::from_utf8(&output.stdout).is_ok(),
                stderr_utf8: std::str::from_utf8(&output.stderr).is_ok(),
                stdout: output.stdout,
                stderr: output.stderr,
            })
            .map_err(|err| {
                format!(
                    "failed to run process `{}`: {err}",
                    runtime_path_string(&resolved)
                )
            })
    }
}

#[derive(Debug)]
pub(crate) struct HostCoreStreamState {
    path: String,
    file: File,
    readable: bool,
    writable: bool,
}

impl HostCoreStreamState {
    pub(crate) fn open_read(policy: &HostCoreFsPolicy, path: &str) -> Result<Self, String> {
        let resolved = policy.resolve_fs_path(path)?;
        let file = File::open(&resolved).map_err(|err| {
            format!(
                "failed to open `{}` for reading: {err}",
                runtime_path_string(&resolved)
            )
        })?;
        Ok(Self {
            path: runtime_path_string(&resolved),
            file,
            readable: true,
            writable: false,
        })
    }

    pub(crate) fn open_write(
        policy: &HostCoreFsPolicy,
        path: &str,
        append: bool,
    ) -> Result<Self, String> {
        let resolved = policy.resolve_fs_path(path)?;
        if let Some(parent) = resolved.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                format!("failed to prepare `{}`: {err}", runtime_path_string(parent))
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
                runtime_path_string(&resolved)
            )
        })?;
        Ok(Self {
            path: runtime_path_string(&resolved),
            file,
            readable: false,
            writable: true,
        })
    }

    pub(crate) fn read(&mut self, max_bytes: usize) -> Result<Vec<u8>, String> {
        if !self.readable {
            return Err(format!(
                "FileStream `{}` is not opened for reading",
                self.path
            ));
        }
        let mut buffer = vec![0u8; max_bytes];
        let read = self
            .file
            .read(&mut buffer)
            .map_err(|err| format!("failed to read from FileStream `{}`: {err}", self.path))?;
        buffer.truncate(read);
        Ok(buffer)
    }

    pub(crate) fn write(&mut self, bytes: &[u8]) -> Result<usize, String> {
        if !self.writable {
            return Err(format!(
                "FileStream `{}` is not opened for writing",
                self.path
            ));
        }
        self.file
            .write_all(bytes)
            .map_err(|err| format!("failed to write to FileStream `{}`: {err}", self.path))?;
        Ok(bytes.len())
    }

    pub(crate) fn eof(&mut self) -> Result<bool, String> {
        if !self.readable {
            return Err(format!(
                "FileStream `{}` is not opened for reading",
                self.path
            ));
        }
        let cursor = self
            .file
            .stream_position()
            .map_err(|err| format!("failed to inspect FileStream `{}`: {err}", self.path))?;
        let len = self
            .file
            .metadata()
            .map_err(|err| format!("failed to stat FileStream `{}`: {err}", self.path))?
            .len();
        Ok(cursor >= len)
    }
}
