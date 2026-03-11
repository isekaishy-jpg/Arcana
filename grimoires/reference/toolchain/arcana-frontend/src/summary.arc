import arcana_frontend.frontend
import std.io

export fn run(target_dir: Str) -> Int:
    let result = arcana_frontend.frontend.check_target :: target_dir :: call
    let error_count = result.0
    let warning_count = 0
    let checksum = result.1

    "CHECK_FINAL_V1" :: :: std.io.print
    error_count :: :: std.io.print
    warning_count :: :: std.io.print
    checksum :: :: std.io.print

    if error_count > 0:
        return 1
    return 0
