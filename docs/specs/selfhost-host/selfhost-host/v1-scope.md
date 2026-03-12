# Arcana Host Platform v2 Scope

This scope freezes the host-core package surface required before selfhost.

Scope notes:
- This file covers host-core packages only.
- It does not define the window/input/canvas or primitive graphics/text app-facing substrate; those remain separate first-party pre-selfhost requirements from `PLAN.md` and `docs/rewrite-roadmap.md`.
- The companion app/runtime substrate contract lives in `docs/specs/selfhost-host/selfhost-host/app-substrate-v1-scope.md`.
- Imported `std` and reference grimoires are behavioral carryover only and must be rebuilt against the rewrite architecture.

## Included
- `std.args`: `count`, `get`
- `std.env`: `has`, `get`, `get_or`
- `std.io`: `print`, `print_line`, `eprint`, `eprint_line`, `flush_stdout`, `flush_stderr`, `read_line`
- `std.path`: `cwd`, `join`, `normalize`, `parent`, `file_name`, `ext`, `is_absolute`, `stem`, `with_ext`, `relative_to`, `canonicalize`, `strip_prefix`
- `std.fs`: `exists`, `is_file`, `is_dir`, `read_text`, `write_text`, `list_dir`, `mkdir_all`, `create_dir`, `remove_file`, `remove_dir`, `remove_dir_all`, `copy_file`, `rename`, `file_size`, `modified_unix_ms`
- `std.fs` binary APIs: `read_bytes`, `write_bytes`
- `std.fs` stream APIs: `stream_open_read`, `stream_open_write`, `stream_read`, `stream_write`, `stream_eof`, `stream_close`
  - stream APIs use an explicit typed `FileStream` handle, not raw `Int` stream ids
  - `stream_close` is a consuming `take` operation and returns `Result[Unit, Str]`
- `std.process`: `exec_status`, `exec_capture`
- `std.bytes`: UTF-8 bytes conversions and explicit byte-array helpers (`len`, `at`, `slice`, `starts_with`, `ends_with`, `find`, `contains`, `concat`, `sha256_hex`, byte-buffer helpers)
- `std.text`: byte-oriented UTF-8 helpers plus explicit search/trim/split/join/repeat/int-parse helpers (`len_bytes`, `byte_at`, `slice_bytes`, `starts_with`, `ends_with`, `find`, `contains`, `split_lines`, `split`, `join`, `trim_start`, `trim_end`, `trim`, `repeat`, `to_int`, `from_int`)
- Native runtime host-root sandbox enforcement for filesystem APIs.
- Native process execution capability gate (`--allow-process` required).
- Rewrite-owned host-core tool proof lane that exercises the approved host surface.
- Rewrite-owned frontend/backend verification proof lane built on the approved host-core surface.

## Excluded
- Network/socket APIs.
- Full Unicode grapheme/text segmentation APIs.
- Additional convenience wrappers outside the included lists, unless later ratified here.
- Compiler-host escape hatches such as `std.process.compiler_compile_*`.
- Window/input/canvas and showcase-facing helper layers.
