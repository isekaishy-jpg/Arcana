import std.fs
import std.path
import std.result
import fs_support
use std.result.Result

export fn write_header(root: Str, report_name: Str) -> Result[Bool, Str]:
    let logs_dir = std.path.join :: root, ".arcana/logs" :: call
    if not (fs_support.mkdir_all_or_false :: logs_dir :: call):
        return Result.Err[Bool, Str] :: "failed to create .arcana/logs" :: call
    let report_path = std.path.join :: logs_dir, report_name :: call
    return std.fs.write_text :: report_path, "Arcana Host Tool MVP v1\n" :: call

export fn write_header_ok(root: Str, report_name: Str) -> Bool:
    let logs_dir = std.path.join :: root, ".arcana/logs" :: call
    if not (fs_support.mkdir_all_or_false :: logs_dir :: call):
        return false
    let report_path = std.path.join :: logs_dir, report_name :: call
    return fs_support.write_text_or_false :: report_path, "Arcana Host Tool MVP v1\n" :: call
