intrinsic fn print[T](read value: T) = IoPrint
intrinsic fn eprint[T](read value: T) = IoEprint
intrinsic fn flush_stdout() = IoFlushStdout
intrinsic fn flush_stderr() = IoFlushStderr
intrinsic fn stdin_read_line_try() -> (Bool, Str) = IoStdinReadLineTry
