import std.result
use std.result.Result

intrinsic fn fs_exists(path: Str) -> Bool = HostFsExists
intrinsic fn fs_is_file(path: Str) -> Bool = HostFsIsFile
intrinsic fn fs_is_dir(path: Str) -> Bool = HostFsIsDir
intrinsic fn fs_read_text(path: Str) -> Result[Str, Str] = HostFsReadTextTry
intrinsic fn fs_read_bytes(path: Str) -> Result[Array[Int], Str] = HostFsReadBytesTry
intrinsic fn fs_write_text(path: Str, text: Str) -> Result[Bool, Str] = HostFsWriteTextTry
intrinsic fn fs_write_bytes(path: Str, read bytes: Array[Int]) -> Result[Bool, Str] = HostFsWriteBytesTry
intrinsic fn fs_stream_open_read(path: Str) -> Result[FileStream, Str] = HostFsStreamOpenReadTry
intrinsic fn fs_stream_open_write(path: Str, append: Bool) -> Result[FileStream, Str] = HostFsStreamOpenWriteTry
intrinsic fn fs_stream_read(read stream: FileStream, max_bytes: Int) -> Result[Array[Int], Str] = HostFsStreamReadTry
intrinsic fn fs_stream_write(read stream: FileStream, read bytes: Array[Int]) -> Result[Int, Str] = HostFsStreamWriteTry
intrinsic fn fs_stream_eof(read stream: FileStream) -> Result[Bool, Str] = HostFsStreamEofTry
intrinsic fn fs_stream_close(read stream: FileStream) -> Result[Unit, Str] = HostFsStreamCloseTry
intrinsic fn fs_list_dir(path: Str) -> Result[List[Str], Str] = HostFsListDirTry
intrinsic fn fs_mkdir_all(path: Str) -> Result[Bool, Str] = HostFsMkdirAllTry
intrinsic fn fs_create_dir(path: Str) -> Result[Bool, Str] = HostFsCreateDirTry
intrinsic fn fs_remove_file(path: Str) -> Result[Bool, Str] = HostFsRemoveFileTry
intrinsic fn fs_remove_dir(path: Str) -> Result[Bool, Str] = HostFsRemoveDirTry
intrinsic fn fs_remove_dir_all(path: Str) -> Result[Bool, Str] = HostFsRemoveDirAllTry
intrinsic fn fs_copy_file(from: Str, to: Str) -> Result[Bool, Str] = HostFsCopyFileTry
intrinsic fn fs_rename(from: Str, to: Str) -> Result[Bool, Str] = HostFsRenameTry
intrinsic fn fs_file_size(path: Str) -> Result[Int, Str] = HostFsFileSizeTry
intrinsic fn fs_modified_unix_ms(path: Str) -> Result[Int, Str] = HostFsModifiedUnixMsTry
