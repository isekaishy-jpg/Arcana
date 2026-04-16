import std.result
use std.result.Result

export record ExecCapture:
    status: Int
    output: (Bytes, Bytes)
    utf8: (Bool, Bool)

// `arcana_process.process` is runtime-owned host-core surface.
export fn exec_status(program: Str, read args: List[Str]) -> Result[Int, Str]:
    return arcana_process.process.exec_status :: program, args :: call

export fn exec_capture(program: Str, read args: List[Str]) -> Result[arcana_process.process.ExecCapture, Str]:
    return arcana_process.process.exec_capture :: program, args :: call

impl ExecCapture:
    fn success(read self: arcana_process.process.ExecCapture) -> Bool:
        return self.status == 0

    fn stdout_text(read self: arcana_process.process.ExecCapture) -> Result[Str, Str]:
        if self.utf8.0:
            return self.output.0 :: :: decode_utf8
        return Result.Err[Str, Str] :: "stdout was not valid UTF-8" :: call

    fn stderr_text(read self: arcana_process.process.ExecCapture) -> Result[Str, Str]:
        if self.utf8.1:
            return self.output.1 :: :: decode_utf8
        return Result.Err[Str, Str] :: "stderr was not valid UTF-8" :: call
