import std.kernel.error
import std.kernel.fs
import std.result
use std.result.Result

export fn exists(path: Str) -> Bool:
    return std.kernel.fs.fs_exists :: path :: call

export fn is_file(path: Str) -> Bool:
    return std.kernel.fs.fs_is_file :: path :: call

export fn is_dir(path: Str) -> Bool:
    return std.kernel.fs.fs_is_dir :: path :: call

export fn read_text(path: Str) -> Result[Str, Str]:
    let pair = std.kernel.fs.fs_read_text_try :: path :: call
    if pair.0:
        return Result.Ok[Str, Str] :: pair.1 :: call
    return Result.Err[Str, Str] :: (std.kernel.error.last_error_take :: :: call) :: call

export fn read_bytes(path: Str) -> Result[Array[Int], Str]:
    let pair = std.kernel.fs.fs_read_bytes_try :: path :: call
    if pair.0:
        return Result.Ok[Array[Int], Str] :: pair.1 :: call
    return Result.Err[Array[Int], Str] :: (std.kernel.error.last_error_take :: :: call) :: call

export fn write_text(path: Str, text: Str) -> Result[Bool, Str]:
    if std.kernel.fs.fs_write_text_try :: path, text :: call:
        return Result.Ok[Bool, Str] :: true :: call
    return Result.Err[Bool, Str] :: (std.kernel.error.last_error_take :: :: call) :: call

export fn write_bytes(path: Str, read bytes: Array[Int]) -> Result[Bool, Str]:
    if std.kernel.fs.fs_write_bytes_try :: path, bytes :: call:
        return Result.Ok[Bool, Str] :: true :: call
    return Result.Err[Bool, Str] :: (std.kernel.error.last_error_take :: :: call) :: call

export fn stream_open_read(path: Str) -> Result[FileStream, Str]:
    let pair = std.kernel.fs.fs_stream_open_read_try :: path :: call
    if pair.0:
        return Result.Ok[FileStream, Str] :: pair.1 :: call
    return Result.Err[FileStream, Str] :: (std.kernel.error.last_error_take :: :: call) :: call

export fn stream_open_write(path: Str, append: Bool) -> Result[FileStream, Str]:
    let pair = std.kernel.fs.fs_stream_open_write_try :: path, append :: call
    if pair.0:
        return Result.Ok[FileStream, Str] :: pair.1 :: call
    return Result.Err[FileStream, Str] :: (std.kernel.error.last_error_take :: :: call) :: call

export fn stream_read(read stream: FileStream, max_bytes: Int) -> Result[Array[Int], Str]:
    let pair = std.kernel.fs.fs_stream_read_try :: stream, max_bytes :: call
    if pair.0:
        return Result.Ok[Array[Int], Str] :: pair.1 :: call
    return Result.Err[Array[Int], Str] :: (std.kernel.error.last_error_take :: :: call) :: call

export fn stream_write(read stream: FileStream, read bytes: Array[Int]) -> Result[Int, Str]:
    let pair = std.kernel.fs.fs_stream_write_try :: stream, bytes :: call
    if pair.0:
        return Result.Ok[Int, Str] :: pair.1 :: call
    return Result.Err[Int, Str] :: (std.kernel.error.last_error_take :: :: call) :: call

export fn stream_eof(read stream: FileStream) -> Result[Bool, Str]:
    let pair = std.kernel.fs.fs_stream_eof_try :: stream :: call
    if pair.0:
        return Result.Ok[Bool, Str] :: pair.1 :: call
    return Result.Err[Bool, Str] :: (std.kernel.error.last_error_take :: :: call) :: call

export fn stream_close(read stream: FileStream) -> Result[Bool, Str]:
    if std.kernel.fs.fs_stream_close_try :: stream :: call:
        return Result.Ok[Bool, Str] :: true :: call
    return Result.Err[Bool, Str] :: (std.kernel.error.last_error_take :: :: call) :: call

export fn list_dir(path: Str) -> Result[List[Str], Str]:
    let pair = std.kernel.fs.fs_list_dir_try :: path :: call
    if pair.0:
        return Result.Ok[List[Str], Str] :: pair.1 :: call
    return Result.Err[List[Str], Str] :: (std.kernel.error.last_error_take :: :: call) :: call

export fn mkdir_all(path: Str) -> Result[Bool, Str]:
    if std.kernel.fs.fs_mkdir_all_try :: path :: call:
        return Result.Ok[Bool, Str] :: true :: call
    return Result.Err[Bool, Str] :: (std.kernel.error.last_error_take :: :: call) :: call

export fn create_dir(path: Str) -> Result[Bool, Str]:
    if std.kernel.fs.fs_create_dir_try :: path :: call:
        return Result.Ok[Bool, Str] :: true :: call
    return Result.Err[Bool, Str] :: (std.kernel.error.last_error_take :: :: call) :: call

export fn remove_file(path: Str) -> Result[Bool, Str]:
    if std.kernel.fs.fs_remove_file_try :: path :: call:
        return Result.Ok[Bool, Str] :: true :: call
    return Result.Err[Bool, Str] :: (std.kernel.error.last_error_take :: :: call) :: call

export fn remove_dir(path: Str) -> Result[Bool, Str]:
    if std.kernel.fs.fs_remove_dir_try :: path :: call:
        return Result.Ok[Bool, Str] :: true :: call
    return Result.Err[Bool, Str] :: (std.kernel.error.last_error_take :: :: call) :: call

export fn remove_dir_all(path: Str) -> Result[Bool, Str]:
    if std.kernel.fs.fs_remove_dir_all_try :: path :: call:
        return Result.Ok[Bool, Str] :: true :: call
    return Result.Err[Bool, Str] :: (std.kernel.error.last_error_take :: :: call) :: call

export fn copy_file(from: Str, to: Str) -> Result[Bool, Str]:
    if std.kernel.fs.fs_copy_file_try :: from, to :: call:
        return Result.Ok[Bool, Str] :: true :: call
    return Result.Err[Bool, Str] :: (std.kernel.error.last_error_take :: :: call) :: call

export fn rename(from: Str, to: Str) -> Result[Bool, Str]:
    if std.kernel.fs.fs_rename_try :: from, to :: call:
        return Result.Ok[Bool, Str] :: true :: call
    return Result.Err[Bool, Str] :: (std.kernel.error.last_error_take :: :: call) :: call

export fn file_size(path: Str) -> Result[Int, Str]:
    let pair = std.kernel.fs.fs_file_size_try :: path :: call
    if pair.0:
        return Result.Ok[Int, Str] :: pair.1 :: call
    return Result.Err[Int, Str] :: (std.kernel.error.last_error_take :: :: call) :: call

export fn modified_unix_ms(path: Str) -> Result[Int, Str]:
    let pair = std.kernel.fs.fs_modified_unix_ms_try :: path :: call
    if pair.0:
        return Result.Ok[Int, Str] :: pair.1 :: call
    return Result.Err[Int, Str] :: (std.kernel.error.last_error_take :: :: call) :: call
