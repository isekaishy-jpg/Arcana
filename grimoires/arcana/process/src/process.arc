import arcana_winapi.helpers.process
import std.result
use std.result.Result

export record ExecCapture:
    status: Int
    output: (Bytes, Bytes)
    utf8: (Bool, Bool)

export fn exec_status(program: Str, read args: List[Str]) -> Result[Int, Str]:
    return arcana_winapi.helpers.process.process_exec_status :: program, args :: call

export fn exec_capture(program: Str, read args: List[Str]) -> Result[arcana_process.process.ExecCapture, Str]:
    let capture = arcana_winapi.helpers.process.process_exec_capture :: program, args :: call
    return match capture:
        Result.Ok(payload) => Result.Ok[arcana_process.process.ExecCapture, Str] :: (arcana_process.process.ExecCapture :: status = payload.0, output = (payload.1.0, payload.1.1.0), utf8 = (payload.1.1.1.0, payload.1.1.1.1) :: call) :: call
        Result.Err(err) => Result.Err[arcana_process.process.ExecCapture, Str] :: err :: call

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
