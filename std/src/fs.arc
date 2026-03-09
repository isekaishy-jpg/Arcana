import std.kernel.host
import std.collections.array
import std.collections.list
import std.result
use std.result.Result

fn host_error() -> Str:
    return std.kernel.host.last_error_take :: :: call

export fn exists(path: Str) -> Bool:
    return std.kernel.host.fs_exists :: path :: call

export fn is_file(path: Str) -> Bool:
    return std.kernel.host.fs_is_file :: path :: call

export fn is_dir(path: Str) -> Bool:
    return std.kernel.host.fs_is_dir :: path :: call

export fn read_text(path: Str) -> Result[Str, Str]:
    let pair = std.kernel.host.fs_read_text_try :: path :: call
    if pair.0:
        return Result.Ok[Str, Str] :: pair.1 :: call
    return Result.Err[Str, Str] :: (std.kernel.host.last_error_take :: :: call) :: call

export fn read_bytes(path: Str) -> Result[Array[Int], Str]:
    let pair = std.kernel.host.fs_read_bytes_try :: path :: call
    if pair.0:
        return Result.Ok[Array[Int], Str] :: pair.1 :: call
    return Result.Err[Array[Int], Str] :: (std.kernel.host.last_error_take :: :: call) :: call

export fn write_text(path: Str, text: Str) -> Result[Bool, Str]:
    if std.kernel.host.fs_write_text_try :: path, text :: call:
        return Result.Ok[Bool, Str] :: true :: call
    return Result.Err[Bool, Str] :: (std.kernel.host.last_error_take :: :: call) :: call

export fn write_bytes(path: Str, read bytes: Array[Int]) -> Result[Bool, Str]:
    if std.kernel.host.fs_write_bytes_try :: path, bytes :: call:
        return Result.Ok[Bool, Str] :: true :: call
    return Result.Err[Bool, Str] :: (std.kernel.host.last_error_take :: :: call) :: call

export fn stream_open_read(path: Str) -> Result[Int, Str]:
    let pair = std.kernel.host.fs_stream_open_read_try :: path :: call
    if pair.0:
        return Result.Ok[Int, Str] :: pair.1 :: call
    return Result.Err[Int, Str] :: (std.kernel.host.last_error_take :: :: call) :: call

export fn stream_open_write(path: Str, append: Bool) -> Result[Int, Str]:
    let pair = std.kernel.host.fs_stream_open_write_try :: path, append :: call
    if pair.0:
        return Result.Ok[Int, Str] :: pair.1 :: call
    return Result.Err[Int, Str] :: (std.kernel.host.last_error_take :: :: call) :: call

export fn stream_read(stream_id: Int, max_bytes: Int) -> Result[Array[Int], Str]:
    let pair = std.kernel.host.fs_stream_read_try :: stream_id, max_bytes :: call
    if pair.0:
        return Result.Ok[Array[Int], Str] :: pair.1 :: call
    return Result.Err[Array[Int], Str] :: (std.kernel.host.last_error_take :: :: call) :: call

export fn stream_write(stream_id: Int, read bytes: Array[Int]) -> Result[Int, Str]:
    let pair = std.kernel.host.fs_stream_write_try :: stream_id, bytes :: call
    if pair.0:
        return Result.Ok[Int, Str] :: pair.1 :: call
    return Result.Err[Int, Str] :: (std.kernel.host.last_error_take :: :: call) :: call

export fn stream_eof(stream_id: Int) -> Result[Bool, Str]:
    let pair = std.kernel.host.fs_stream_eof_try :: stream_id :: call
    if pair.0:
        return Result.Ok[Bool, Str] :: pair.1 :: call
    return Result.Err[Bool, Str] :: (std.kernel.host.last_error_take :: :: call) :: call

export fn stream_close(stream_id: Int) -> Result[Bool, Str]:
    if std.kernel.host.fs_stream_close_try :: stream_id :: call:
        return Result.Ok[Bool, Str] :: true :: call
    return Result.Err[Bool, Str] :: (std.kernel.host.last_error_take :: :: call) :: call

export fn list_dir(path: Str) -> Result[List[Str], Str]:
    let pair = std.kernel.host.fs_list_dir_try :: path :: call
    if pair.0:
        return Result.Ok[List[Str], Str] :: pair.1 :: call
    return Result.Err[List[Str], Str] :: (std.kernel.host.last_error_take :: :: call) :: call

export fn mkdir_all(path: Str) -> Result[Bool, Str]:
    if std.kernel.host.fs_mkdir_all_try :: path :: call:
        return Result.Ok[Bool, Str] :: true :: call
    return Result.Err[Bool, Str] :: (std.kernel.host.last_error_take :: :: call) :: call

export fn remove_file(path: Str) -> Result[Bool, Str]:
    if std.kernel.host.fs_remove_file_try :: path :: call:
        return Result.Ok[Bool, Str] :: true :: call
    return Result.Err[Bool, Str] :: (std.kernel.host.last_error_take :: :: call) :: call

export fn remove_dir_all(path: Str) -> Result[Bool, Str]:
    if std.kernel.host.fs_remove_dir_all_try :: path :: call:
        return Result.Ok[Bool, Str] :: true :: call
    return Result.Err[Bool, Str] :: (std.kernel.host.last_error_take :: :: call) :: call

export fn read_text_or(path: Str, fallback: Str) -> Str:
    let pair = std.kernel.host.fs_read_text_try :: path :: call
    if pair.0:
        return pair.1
    return fallback

export fn list_dir_or_empty(path: Str) -> List[Str]:
    let pair = std.kernel.host.fs_list_dir_try :: path :: call
    if pair.0:
        return pair.1
    return std.collections.list.new[Str] :: :: call

export fn mkdir_all_or_false(path: Str) -> Bool:
    return std.kernel.host.fs_mkdir_all_try :: path :: call

export fn write_text_or_false(path: Str, text: Str) -> Bool:
    return std.kernel.host.fs_write_text_try :: path, text :: call

export fn read_bytes_or_empty(path: Str) -> Array[Int]:
    let pair = std.kernel.host.fs_read_bytes_try :: path :: call
    if pair.0:
        return pair.1
    return std.collections.array.new[Int] :: 0, 0 :: call

export fn write_bytes_or_false(path: Str, read bytes: Array[Int]) -> Bool:
    return std.kernel.host.fs_write_bytes_try :: path, bytes :: call
