import std.kernel.fs
import std.cleanup
import std.result
use std.result.Result

export opaque type FileStream as move, boundary_unsafe

lang file_stream_handle = FileStream

export fn exists(path: Str) -> Bool:
    return std.kernel.fs.fs_exists :: path :: call

export fn is_file(path: Str) -> Bool:
    return std.kernel.fs.fs_is_file :: path :: call

export fn is_dir(path: Str) -> Bool:
    return std.kernel.fs.fs_is_dir :: path :: call

export fn read_text(path: Str) -> Result[Str, Str]:
    return std.kernel.fs.fs_read_text :: path :: call

export fn read_bytes(path: Str) -> Result[Array[Int], Str]:
    return std.kernel.fs.fs_read_bytes :: path :: call

export fn write_text(path: Str, text: Str) -> Result[Unit, Str]:
    return std.kernel.fs.fs_write_text :: path, text :: call

export fn write_bytes(path: Str, read bytes: Array[Int]) -> Result[Unit, Str]:
    return std.kernel.fs.fs_write_bytes :: path, bytes :: call

export fn stream_open_read(path: Str) -> Result[FileStream, Str]:
    return std.kernel.fs.fs_stream_open_read :: path :: call

export fn stream_open_write(path: Str, append: Bool) -> Result[FileStream, Str]:
    return std.kernel.fs.fs_stream_open_write :: path, append :: call

export fn stream_read(edit stream: FileStream, max_bytes: Int) -> Result[Array[Int], Str]:
    return std.kernel.fs.fs_stream_read :: stream, max_bytes :: call

export fn stream_write(edit stream: FileStream, read bytes: Array[Int]) -> Result[Int, Str]:
    return std.kernel.fs.fs_stream_write :: stream, bytes :: call

export fn stream_eof(read stream: FileStream) -> Result[Bool, Str]:
    return std.kernel.fs.fs_stream_eof :: stream :: call

export fn stream_close(take stream: FileStream) -> Result[Unit, Str]:
    return std.kernel.fs.fs_stream_close :: stream :: call

export fn list_dir(path: Str) -> Result[List[Str], Str]:
    return std.kernel.fs.fs_list_dir :: path :: call

export fn mkdir_all(path: Str) -> Result[Unit, Str]:
    return std.kernel.fs.fs_mkdir_all :: path :: call

export fn create_dir(path: Str) -> Result[Unit, Str]:
    return std.kernel.fs.fs_create_dir :: path :: call

export fn remove_file(path: Str) -> Result[Unit, Str]:
    return std.kernel.fs.fs_remove_file :: path :: call

export fn remove_dir(path: Str) -> Result[Unit, Str]:
    return std.kernel.fs.fs_remove_dir :: path :: call

export fn remove_dir_all(path: Str) -> Result[Unit, Str]:
    return std.kernel.fs.fs_remove_dir_all :: path :: call

export fn copy_file(from: Str, to: Str) -> Result[Unit, Str]:
    return std.kernel.fs.fs_copy_file :: from, to :: call

export fn rename(from: Str, to: Str) -> Result[Unit, Str]:
    return std.kernel.fs.fs_rename :: from, to :: call

export fn file_size(path: Str) -> Result[Int, Str]:
    return std.kernel.fs.fs_file_size :: path :: call

export fn modified_unix_ms(path: Str) -> Result[Int, Str]:
    return std.kernel.fs.fs_modified_unix_ms :: path :: call

impl std.cleanup.Cleanup[std.fs.FileStream] for std.fs.FileStream:
    fn cleanup(take self: std.fs.FileStream) -> Result[Unit, Str]:
        return std.fs.stream_close :: self :: call
