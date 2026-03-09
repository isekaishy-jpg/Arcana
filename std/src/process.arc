import std.kernel.host
import std.bytes
import std.result
use std.result.Result

record ExecCapture:
    status: Int
    streams: (Array[Int], Array[Int])
    truncated: (Bool, Bool)

fn host_error() -> Str:
    return std.kernel.host.last_error_take :: :: call

export fn exec_status(program: Str, read args: List[Str]) -> Result[Int, Str]:
    let pair = std.kernel.host.process_exec_status_try :: program, args :: call
    if pair.0:
        return Result.Ok[Int, Str] :: pair.1 :: call
    return Result.Err[Int, Str] :: (std.kernel.host.last_error_take :: :: call) :: call

export fn exec_status_try(program: Str, read args: List[Str]) -> (Bool, Int):
    return std.kernel.host.process_exec_status_try :: program, args :: call

export fn exec_capture(program: Str, read args: List[Str]) -> Result[ExecCapture, Str]:
    let pair = std.kernel.host.process_exec_capture_try :: program, args :: call
    if not pair.0:
        return Result.Err[ExecCapture, Str] :: (std.kernel.host.last_error_take :: :: call) :: call
    let payload = pair.1
    let status = payload.0
    let streams = (payload.1.0, payload.1.1.0)
    let truncated = (payload.1.1.1.0, payload.1.1.1.1)
    let capture = std.process.ExecCapture :: status = status, streams = streams, truncated = truncated :: call
    return Result.Ok[ExecCapture, Str] :: capture :: call

export fn exec_capture_try(program: Str, read args: List[Str]) -> (Bool, ExecCapture):
    let pair = std.kernel.host.process_exec_capture_try :: program, args :: call
    let payload = pair.1
    let status = payload.0
    let streams = (payload.1.0, payload.1.1.0)
    let truncated = (payload.1.1.1.0, payload.1.1.1.1)
    let capture = std.process.ExecCapture :: status = status, streams = streams, truncated = truncated :: call
    return (pair.0, capture)

export fn exec_capture_text_try(program: Str, read args: List[Str]) -> (Bool, (Int, (Str, Str))):
    let pair = std.process.exec_capture_try :: program, args :: call
    if not pair.0:
        return (false, (0, ("", "")))
    let capture = pair.1
    let status = capture.status
    let stdout_text = std.bytes.to_str_utf8 :: capture.streams.0 :: call
    let stderr_text = std.bytes.to_str_utf8 :: capture.streams.1 :: call
    return (true, (status, (stdout_text, stderr_text)))

export fn compiler_compile_try(source: Str, out: Str) -> Bool:
    return std.kernel.host.compiler_compile_try :: source, out :: call

export fn compiler_compile_lib_try(source: Str, out: Str) -> Bool:
    return std.kernel.host.compiler_compile_lib_try :: source, out :: call

export fn last_error_take() -> Str:
    return std.kernel.host.last_error_take :: :: call
