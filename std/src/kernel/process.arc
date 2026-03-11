intrinsic fn process_exec_status_try(program: Str, read args: List[Str]) -> (Bool, Int) = HostProcessExecStatusTry
intrinsic fn process_exec_capture_try(program: Str, read args: List[Str]) -> (Bool, (Int, (Array[Int], (Array[Int], (Bool, Bool))))) = HostProcessExecCaptureTry
