import std.kernel.error
import std.kernel.process
import std.bytes
import std.result
use std.result.Result

export record ExecCapture:
    status: Int
    output: (Array[Int], Array[Int])
    utf8: (Bool, Bool)

export fn exec_status(program: Str, read args: List[Str]) -> Result[Int, Str]:
    let pair = std.kernel.process.process_exec_status_try :: program, args :: call
    if pair.0:
        return Result.Ok[Int, Str] :: pair.1 :: call
    return Result.Err[Int, Str] :: (std.kernel.error.last_error_take :: :: call) :: call

export fn exec_capture(program: Str, read args: List[Str]) -> Result[std.process.ExecCapture, Str]:
    let pair = std.kernel.process.process_exec_capture_try :: program, args :: call
    if pair.0:
        let payload = pair.1
        let capture = std.process.ExecCapture :: status = payload.0, output = (payload.1.0, payload.1.1.0), utf8 = (payload.1.1.1.0, payload.1.1.1.1) :: call
        return Result.Ok[std.process.ExecCapture, Str] :: capture :: call
    return Result.Err[std.process.ExecCapture, Str] :: (std.kernel.error.last_error_take :: :: call) :: call

impl ExecCapture:
    fn success(read self: std.process.ExecCapture) -> Bool:
        return self.status == 0

    fn stdout_text(read self: std.process.ExecCapture) -> Result[Str, Str]:
        if self.utf8.0:
            return Result.Ok[Str, Str] :: (std.bytes.to_str_utf8 :: self.output.0 :: call) :: call
        return Result.Err[Str, Str] :: "stdout was not valid UTF-8" :: call

    fn stderr_text(read self: std.process.ExecCapture) -> Result[Str, Str]:
        if self.utf8.1:
            return Result.Ok[Str, Str] :: (std.bytes.to_str_utf8 :: self.output.1 :: call) :: call
        return Result.Err[Str, Str] :: "stderr was not valid UTF-8" :: call
