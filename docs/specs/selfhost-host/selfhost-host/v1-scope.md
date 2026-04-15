# Arcana Host Platform v2 Scope

This scope freezes the host-core package surface required before selfhost.

Scope notes:
- This file covers host-core packages only.
- It does not define any future window/input/graphics app-facing layer; any such layer requires a separate scope if it returns.
- Imported `std` and reference grimoires are behavioral carryover only and must be rebuilt against the rewrite architecture.

## Included
- `arcana_process.args`: `count`, `get`
- `arcana_process.env`: `has`, `get`, `get_or`
- `arcana_process.io`: `print`, `print_line`, `eprint`, `eprint_line`, `flush_stdout`, `flush_stderr`, `read_line`
- `arcana_process.path`: `cwd`, `join`, `normalize`, `parent`, `file_name`, `ext`, `is_absolute`, `stem`, `with_ext`, `relative_to`, `canonicalize`, `strip_prefix`
- `arcana_process.fs`: `exists`, `is_file`, `is_dir`, `read_text`, `write_text`, `list_dir`, `mkdir_all`, `create_dir`, `remove_file`, `remove_dir`, `remove_dir_all`, `copy_file`, `rename`, `file_size`, `modified_unix_ms`
- `arcana_process.fs` binary APIs: `read_bytes`, `write_bytes`
- `arcana_process.fs` stream APIs: `stream_open_read`, `stream_open_write`, `stream_read`, `stream_write`, `stream_eof`, `stream_close`
  - stream APIs use an explicit typed `FileStream` handle, not raw `Int` stream ids
  - canonical handle path: `arcana_winapi.process_handles.FileStream`
  - `stream_close` is a consuming `take` operation and returns `Result[Unit, Str]`
- `arcana_process.process`: `exec_status`, `exec_capture`
- core binary/text payloads:
  - `Bytes`
  - `ByteBuffer`
  - `Utf16`
  - `Utf16Buffer`
- `std.text`: UTF-8 / UTF-16 conversions plus explicit byte/text helpers (`bytes_from_str_utf8`, `bytes_to_str_utf8`, `bytes_len`, `bytes_at`, `bytes_slice`, `bytes_sha256_hex`, `utf16_len`, `utf16_at`, `utf16_slice`, `starts_with`, `ends_with`, `find`, `contains`, `split_lines`, `split`, `join`, `trim_start`, `trim_end`, `trim`, `repeat`, `to_int`, `from_int`)
- Native runtime host-root sandbox enforcement for filesystem APIs.
- Native process execution capability gate (`--allow-process` required).
- Rewrite-owned host-core tool proof lane that exercises the approved host surface.
- Rewrite-owned frontend/backend verification proof lane built on the approved host-core surface.

## Excluded
- Network/socket APIs.
- Full Unicode grapheme/text segmentation APIs.
- Additional convenience wrappers outside the included lists, unless later ratified here.
- Compiler-host escape hatches such as `arcana_process.process.compiler_compile_*`.
- Window/input/canvas and showcase-facing helper layers.
