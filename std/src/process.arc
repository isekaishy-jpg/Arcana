import std.kernel.host
import std.result
use std.result.Result

export fn exec_status(program: Str, read args: List[Str]) -> Result[Int, Str]:
    let pair = std.kernel.host.process_exec_status_try :: program, args :: call
    if pair.0:
        return Result.Ok[Int, Str] :: pair.1 :: call
    return Result.Err[Int, Str] :: (std.kernel.host.last_error_take :: :: call) :: call
