import std.binary
import std.collections.list
import std.result
import std.text
use arcana_winapi.process_handles.FileStream
use std.result.Result

native fn take_last_error() -> Str = helpers.process.take_last_error
native fn path_stem_raw(path: Str) -> Str = helpers.process.path_stem
native fn path_relative_to_raw(path: Str, base: Str) -> Str = helpers.process.path_relative_to
native fn path_canonicalize_raw(path: Str) -> Str = helpers.process.path_canonicalize
native fn path_strip_prefix_raw(path: Str, prefix: Str) -> Str = helpers.process.path_strip_prefix
native fn fs_read_text_raw(path: Str) -> Str = helpers.process.fs_read_text
native fn fs_read_bytes_raw(path: Str) -> Bytes = helpers.process.fs_read_bytes
native fn fs_write_text_raw(path: Str, text: Str) -> Bool = helpers.process.fs_write_text
native fn fs_write_bytes_raw(path: Str, read bytes: Bytes) -> Bool = helpers.process.fs_write_bytes
native fn fs_stream_open_read_raw(path: Str) -> FileStream = helpers.process.fs_stream_open_read
native fn fs_stream_open_write_raw(path: Str, append: Bool) -> FileStream = helpers.process.fs_stream_open_write
native fn fs_stream_read_raw(edit stream: FileStream, max_bytes: Int) -> Bytes = helpers.process.fs_stream_read
native fn fs_stream_write_raw(edit stream: FileStream, read bytes: Bytes) -> Int = helpers.process.fs_stream_write
native fn fs_stream_eof_raw(read stream: FileStream) -> Bool = helpers.process.fs_stream_eof
native fn fs_stream_close_raw(take stream: FileStream) -> Bool = helpers.process.fs_stream_close
native fn fs_list_dir_raw(path: Str) -> Bytes = helpers.process.fs_list_dir
native fn fs_mkdir_all_raw(path: Str) -> Bool = helpers.process.fs_mkdir_all
native fn fs_create_dir_raw(path: Str) -> Bool = helpers.process.fs_create_dir
native fn fs_remove_file_raw(path: Str) -> Bool = helpers.process.fs_remove_file
native fn fs_remove_dir_raw(path: Str) -> Bool = helpers.process.fs_remove_dir
native fn fs_remove_dir_all_raw(path: Str) -> Bool = helpers.process.fs_remove_dir_all
native fn fs_copy_file_raw(from: Str, to: Str) -> Bool = helpers.process.fs_copy_file
native fn fs_rename_raw(from: Str, to: Str) -> Bool = helpers.process.fs_rename
native fn fs_file_size_raw(path: Str) -> Int = helpers.process.fs_file_size
native fn fs_modified_unix_ms_raw(path: Str) -> Int = helpers.process.fs_modified_unix_ms
native fn process_exec_status_raw(program: Str, read args: Bytes) -> Int = helpers.process.process_exec_status
native fn process_exec_capture_raw(program: Str, read args: Bytes) -> Bytes = helpers.process.process_exec_capture

fn result_unit(ok: Bool) -> Result[Unit, Str]:
    if ok:
        return Result.Ok[Unit, Str] :: :: call
    return Result.Err[Unit, Str] :: (take_last_error :: :: call) :: call

fn result_str(value: Str) -> Result[Str, Str]:
    let err = take_last_error :: :: call
    if err == "":
        return Result.Ok[Str, Str] :: value :: call
    return Result.Err[Str, Str] :: err :: call

fn result_bytes(value: Bytes) -> Result[Bytes, Str]:
    let err = take_last_error :: :: call
    if err == "":
        return Result.Ok[Bytes, Str] :: value :: call
    return Result.Err[Bytes, Str] :: err :: call

fn result_stream(take value: FileStream) -> Result[FileStream, Str]:
    let err = take_last_error :: :: call
    if err == "":
        return Result.Ok[FileStream, Str] :: value :: call
    return Result.Err[FileStream, Str] :: err :: call

fn result_int(value: Int) -> Result[Int, Str]:
    let err = take_last_error :: :: call
    if err == "":
        return Result.Ok[Int, Str] :: value :: call
    return Result.Err[Int, Str] :: err :: call

fn result_bool(value: Bool) -> Result[Bool, Str]:
    let err = take_last_error :: :: call
    if err == "":
        return Result.Ok[Bool, Str] :: value :: call
    return Result.Err[Bool, Str] :: err :: call

fn push_u32_le(edit writer: std.binary.Writer, value: Int):
    writer :: (value & 255) :: push_u8
    writer :: ((value shr 8) & 255) :: push_u8
    writer :: ((value shr 16) & 255) :: push_u8
    writer :: ((value shr 24) & 255) :: push_u8

fn push_i32_le(edit writer: std.binary.Writer, value: Int):
    let mut raw = value
    if raw < 0:
        raw += 4294967296
    push_u32_le :: writer, raw :: call

fn read_u32_le(read payload: Bytes, index: Int) -> Int:
    let a = std.text.bytes_at :: payload, index :: call
    let b = (std.text.bytes_at :: payload, index + 1 :: call) << 8
    let c = (std.text.bytes_at :: payload, index + 2 :: call) << 16
    let d = (std.text.bytes_at :: payload, index + 3 :: call) << 24
    return a | b | c | d

fn read_i32_le(read payload: Bytes, index: Int) -> Int:
    let raw = read_u32_le :: payload, index :: call
    if raw >= 2147483648:
        return raw - 4294967296
    return raw

fn encode_string_list(read values: List[Str]) -> Bytes:
    let mut writer = std.binary.writer :: :: call
    let mut count = 0
    for _ in values:
        count += 1
    push_u32_le :: writer, count :: call
    for value in values:
        let encoded = value :: :: encode_utf8
        let len = encoded :: :: len
        push_u32_le :: writer, len :: call
        let mut index = 0
        while index < len:
            writer :: (std.text.bytes_at :: encoded, index :: call) :: push_u8
            index += 1
    return writer :: :: into_bytes

fn decode_string_list(read payload: Bytes) -> Result[List[Str], Str]:
    let total = payload :: :: len
    if total < 4:
        return Result.Err[List[Str], Str] :: "string list payload truncated" :: call
    let count = read_u32_le :: payload, 0 :: call
    let mut cursor = 4
    let mut index = 0
    let mut out = std.collections.list.new[Str] :: :: call
    while index < count:
        if cursor + 4 > total:
            return Result.Err[List[Str], Str] :: "string list payload truncated" :: call
        let len = read_u32_le :: payload, cursor :: call
        cursor += 4
        if len < 0 or cursor + len > total:
            return Result.Err[List[Str], Str] :: "string list payload length out of range" :: call
        let text_bytes = std.text.bytes_slice :: payload, cursor, cursor + len :: call
        let text = text_bytes :: :: decode_utf8
        if text :: :: is_err:
            return Result.Err[List[Str], Str] :: "string list payload contained invalid utf-8" :: call
        out :: (text :: "" :: unwrap_or) :: push
        cursor += len
        index += 1
    if cursor != total:
        return Result.Err[List[Str], Str] :: "string list payload has trailing bytes" :: call
    return Result.Ok[List[Str], Str] :: out :: call

fn decode_exec_capture_payload(read payload: Bytes) -> Result[(Int, (Bytes, (Bytes, (Bool, Bool)))), Str]:
    let total = payload :: :: len
    if total < 14:
        return Result.Err[(Int, (Bytes, (Bytes, (Bool, Bool)))), Str] :: "exec capture payload truncated" :: call
    let status = read_i32_le :: payload, 0 :: call
    let stdout_utf8 = (std.text.bytes_at :: payload, 4 :: call) != 0
    let stderr_utf8 = (std.text.bytes_at :: payload, 5 :: call) != 0
    let stdout_len = read_u32_le :: payload, 6 :: call
    let stdout_start = 10
    let stdout_end = stdout_start + stdout_len
    if stdout_len < 0 or stdout_end + 4 > total:
        return Result.Err[(Int, (Bytes, (Bytes, (Bool, Bool)))), Str] :: "exec capture stdout payload truncated" :: call
    let stderr_len = read_u32_le :: payload, stdout_end :: call
    let stderr_start = stdout_end + 4
    let stderr_end = stderr_start + stderr_len
    if stderr_len < 0 or stderr_end != total:
        return Result.Err[(Int, (Bytes, (Bytes, (Bool, Bool)))), Str] :: "exec capture stderr payload truncated" :: call
    let stdout = std.text.bytes_slice :: payload, stdout_start, stdout_end :: call
    let stderr = std.text.bytes_slice :: payload, stderr_start, stderr_end :: call
    return Result.Ok[(Int, (Bytes, (Bytes, (Bool, Bool)))), Str] :: (status, (stdout, (stderr, (stdout_utf8, stderr_utf8)))) :: call

export native fn arg_count() -> Int = helpers.process.arg_count
export native fn arg_get(index: Int) -> Str = helpers.process.arg_get
export native fn env_has(name: Str) -> Bool = helpers.process.env_has
export native fn env_get(name: Str) -> Str = helpers.process.env_get
export native fn path_cwd() -> Str = helpers.process.path_cwd
export native fn path_join(a: Str, b: Str) -> Str = helpers.process.path_join
export native fn path_normalize(path: Str) -> Str = helpers.process.path_normalize
export native fn path_parent(path: Str) -> Str = helpers.process.path_parent
export native fn path_file_name(path: Str) -> Str = helpers.process.path_file_name
export native fn path_ext(path: Str) -> Str = helpers.process.path_ext
export native fn path_is_absolute(path: Str) -> Bool = helpers.process.path_is_absolute

export fn path_stem(path: Str) -> Result[Str, Str]:
    return result_str :: (path_stem_raw :: path :: call) :: call

export native fn path_with_ext(path: Str, ext: Str) -> Str = helpers.process.path_with_ext

export fn path_relative_to(path: Str, base: Str) -> Result[Str, Str]:
    return result_str :: (path_relative_to_raw :: path, base :: call) :: call

export fn path_canonicalize(path: Str) -> Result[Str, Str]:
    return result_str :: (path_canonicalize_raw :: path :: call) :: call

export fn path_strip_prefix(path: Str, prefix: Str) -> Result[Str, Str]:
    return result_str :: (path_strip_prefix_raw :: path, prefix :: call) :: call

export native fn fs_exists(path: Str) -> Bool = helpers.process.fs_exists
export native fn fs_is_file(path: Str) -> Bool = helpers.process.fs_is_file
export native fn fs_is_dir(path: Str) -> Bool = helpers.process.fs_is_dir

export fn fs_read_text(path: Str) -> Result[Str, Str]:
    return result_str :: (fs_read_text_raw :: path :: call) :: call

export fn fs_read_bytes(path: Str) -> Result[Bytes, Str]:
    return result_bytes :: (fs_read_bytes_raw :: path :: call) :: call

export fn fs_write_text(path: Str, text: Str) -> Result[Unit, Str]:
    return result_unit :: (fs_write_text_raw :: path, text :: call) :: call

export fn fs_write_bytes(path: Str, read bytes: Bytes) -> Result[Unit, Str]:
    return result_unit :: (fs_write_bytes_raw :: path, bytes :: call) :: call

export fn fs_stream_open_read(path: Str) -> Result[FileStream, Str]:
    return result_stream :: (fs_stream_open_read_raw :: path :: call) :: call

export fn fs_stream_open_write(path: Str, append: Bool) -> Result[FileStream, Str]:
    return result_stream :: (fs_stream_open_write_raw :: path, append :: call) :: call

export fn fs_stream_read(edit stream: FileStream, max_bytes: Int) -> Result[Bytes, Str]:
    return result_bytes :: (fs_stream_read_raw :: stream, max_bytes :: call) :: call

export fn fs_stream_write(edit stream: FileStream, read bytes: Bytes) -> Result[Int, Str]:
    return result_int :: (fs_stream_write_raw :: stream, bytes :: call) :: call

export fn fs_stream_eof(read stream: FileStream) -> Result[Bool, Str]:
    return result_bool :: (fs_stream_eof_raw :: stream :: call) :: call

export fn fs_stream_close(take stream: FileStream) -> Result[Unit, Str]:
    return result_unit :: (fs_stream_close_raw :: stream :: call) :: call

export fn fs_list_dir(path: Str) -> Result[List[Str], Str]:
    let payload = fs_list_dir_raw :: path :: call
    let err = take_last_error :: :: call
    if err != "":
        return Result.Err[List[Str], Str] :: err :: call
    return decode_string_list :: payload :: call

export fn fs_mkdir_all(path: Str) -> Result[Unit, Str]:
    return result_unit :: (fs_mkdir_all_raw :: path :: call) :: call

export fn fs_create_dir(path: Str) -> Result[Unit, Str]:
    return result_unit :: (fs_create_dir_raw :: path :: call) :: call

export fn fs_remove_file(path: Str) -> Result[Unit, Str]:
    return result_unit :: (fs_remove_file_raw :: path :: call) :: call

export fn fs_remove_dir(path: Str) -> Result[Unit, Str]:
    return result_unit :: (fs_remove_dir_raw :: path :: call) :: call

export fn fs_remove_dir_all(path: Str) -> Result[Unit, Str]:
    return result_unit :: (fs_remove_dir_all_raw :: path :: call) :: call

export fn fs_copy_file(from: Str, to: Str) -> Result[Unit, Str]:
    return result_unit :: (fs_copy_file_raw :: from, to :: call) :: call

export fn fs_rename(from: Str, to: Str) -> Result[Unit, Str]:
    return result_unit :: (fs_rename_raw :: from, to :: call) :: call

export fn fs_file_size(path: Str) -> Result[Int, Str]:
    return result_int :: (fs_file_size_raw :: path :: call) :: call

export fn fs_modified_unix_ms(path: Str) -> Result[Int, Str]:
    return result_int :: (fs_modified_unix_ms_raw :: path :: call) :: call

export fn process_exec_status(program: Str, read args: List[Str]) -> Result[Int, Str]:
    return result_int :: (process_exec_status_raw :: program, (encode_string_list :: args :: call) :: call) :: call

export fn process_exec_capture(program: Str, read args: List[Str]) -> Result[(Int, (Bytes, (Bytes, (Bool, Bool)))), Str]:
    let payload = process_exec_capture_raw :: program, (encode_string_list :: args :: call) :: call
    let err = take_last_error :: :: call
    if err != "":
        return Result.Err[(Int, (Bytes, (Bytes, (Bool, Bool)))), Str] :: err :: call
    return decode_exec_capture_payload :: payload :: call

