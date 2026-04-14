use std::io::{BufRead, Write};
use std::time::{Duration, Instant};

use crate::RuntimeCoreHost;

pub(crate) struct ProcessRuntimeHost {
    start: Instant,
}

impl ProcessRuntimeHost {
    pub(crate) fn current_process() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    fn elapsed(&self) -> Duration {
        self.start.elapsed()
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
}
