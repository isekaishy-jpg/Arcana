import std.cleanup
import std.result
use std.result.Result

export opaque type FileStream as move, boundary_unsafe

// `arcana_process.fs` is runtime-owned host-core surface.
export fn exists(path: Str) -> Bool:
    return arcana_process.fs.exists :: path :: call

export fn is_file(path: Str) -> Bool:
    return arcana_process.fs.is_file :: path :: call

export fn is_dir(path: Str) -> Bool:
    return arcana_process.fs.is_dir :: path :: call

export fn read_text(path: Str) -> Result[Str, Str]:
    return arcana_process.fs.read_text :: path :: call

export fn read_bytes(path: Str) -> Result[Bytes, Str]:
    return arcana_process.fs.read_bytes :: path :: call

export fn write_text(path: Str, text: Str) -> Result[Unit, Str]:
    return arcana_process.fs.write_text :: path, text :: call

export fn write_bytes(path: Str, read bytes: Bytes) -> Result[Unit, Str]:
    return arcana_process.fs.write_bytes :: path, bytes :: call

export fn stream_open_read(path: Str) -> Result[FileStream, Str]:
    return arcana_process.fs.stream_open_read :: path :: call

export fn stream_open_write(path: Str, append: Bool) -> Result[FileStream, Str]:
    return arcana_process.fs.stream_open_write :: path, append :: call

export fn stream_read(edit stream: FileStream, max_bytes: Int) -> Result[Bytes, Str]:
    return arcana_process.fs.stream_read :: stream, max_bytes :: call

export fn stream_write(edit stream: FileStream, read bytes: Bytes) -> Result[Int, Str]:
    return arcana_process.fs.stream_write :: stream, bytes :: call

export fn stream_eof(read stream: FileStream) -> Result[Bool, Str]:
    return arcana_process.fs.stream_eof :: stream :: call

export fn stream_close(take stream: FileStream) -> Result[Unit, Str]:
    return arcana_process.fs.stream_close :: stream :: call

export fn list_dir(path: Str) -> Result[List[Str], Str]:
    return arcana_process.fs.list_dir :: path :: call

export fn mkdir_all(path: Str) -> Result[Unit, Str]:
    return arcana_process.fs.mkdir_all :: path :: call

export fn create_dir(path: Str) -> Result[Unit, Str]:
    return arcana_process.fs.create_dir :: path :: call

export fn remove_file(path: Str) -> Result[Unit, Str]:
    return arcana_process.fs.remove_file :: path :: call

export fn remove_dir(path: Str) -> Result[Unit, Str]:
    return arcana_process.fs.remove_dir :: path :: call

export fn remove_dir_all(path: Str) -> Result[Unit, Str]:
    return arcana_process.fs.remove_dir_all :: path :: call

export fn copy_file(from: Str, to: Str) -> Result[Unit, Str]:
    return arcana_process.fs.copy_file :: from, to :: call

export fn rename(from: Str, to: Str) -> Result[Unit, Str]:
    return arcana_process.fs.rename :: from, to :: call

export fn file_size(path: Str) -> Result[Int, Str]:
    return arcana_process.fs.file_size :: path :: call

export fn modified_unix_ms(path: Str) -> Result[Int, Str]:
    return arcana_process.fs.modified_unix_ms :: path :: call

impl std.cleanup.Cleanup[arcana_process.fs.FileStream] for arcana_process.fs.FileStream:
    fn cleanup(take self: arcana_process.fs.FileStream) -> Result[Unit, Str]:
        return arcana_process.fs.stream_close :: self :: call

