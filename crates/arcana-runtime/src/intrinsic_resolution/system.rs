use crate::runtime_intrinsics::RuntimeIntrinsic;

pub(super) fn resolve_path(parts: &[&str]) -> Option<RuntimeIntrinsic> {
    match parts {
        ["std", "kernel", "io", "print"] => Some(RuntimeIntrinsic::IoPrint),
        ["std", "kernel", "io", "eprint"] => Some(RuntimeIntrinsic::IoEprint),
        ["std", "kernel", "io", "flush_stdout"] => Some(RuntimeIntrinsic::IoFlushStdout),
        ["std", "kernel", "io", "flush_stderr"] => Some(RuntimeIntrinsic::IoFlushStderr),
        ["std", "kernel", "io", "stdin_read_line"] => Some(RuntimeIntrinsic::IoStdinReadLineTry),
        ["std", "time", "monotonic_now_ms"] => Some(RuntimeIntrinsic::StdTimeMonotonicNowMs),
        ["std", "kernel", "time", "monotonic_now_ms"] => Some(RuntimeIntrinsic::TimeMonotonicNowMs),
        ["std", "time", "monotonic_now_ns"] | ["std", "kernel", "time", "monotonic_now_ns"] => {
            Some(RuntimeIntrinsic::TimeMonotonicNowNs)
        }
        _ => None,
    }
}

pub(super) fn resolve_impl(intrinsic_impl: &str) -> Option<RuntimeIntrinsic> {
    match intrinsic_impl {
        "IoPrint" => Some(RuntimeIntrinsic::IoPrint),
        "IoEprint" => Some(RuntimeIntrinsic::IoEprint),
        "IoFlushStdout" => Some(RuntimeIntrinsic::IoFlushStdout),
        "IoFlushStderr" => Some(RuntimeIntrinsic::IoFlushStderr),
        "IoStdinReadLineTry" => Some(RuntimeIntrinsic::IoStdinReadLineTry),
        "PackageCurrentAssetRootTry" => Some(RuntimeIntrinsic::PackageAssetRootTry),
        "HostTimeMonotonicNowMs" => Some(RuntimeIntrinsic::TimeMonotonicNowMs),
        "HostTimeMonotonicNowNs" => Some(RuntimeIntrinsic::TimeMonotonicNowNs),
        _ => None,
    }
}
