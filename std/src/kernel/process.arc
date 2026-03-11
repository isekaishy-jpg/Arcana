import std.result
use std.result.Result

intrinsic fn process_exec_status(program: Str, read args: List[Str]) -> Result[Int, Str] = HostProcessExecStatusTry
intrinsic fn process_exec_capture(program: Str, read args: List[Str]) -> Result[(Int, (Array[Int], (Array[Int], (Bool, Bool)))), Str] = HostProcessExecCaptureTry
