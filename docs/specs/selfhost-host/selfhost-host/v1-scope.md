# Arcana Host Platform v2 Scope

Implemented through Plan 42:

## Included
- `std.args`: `count`, `get`
- `std.env`: `has`, `get`, `get_or`
- `std.path`: `cwd`, `join`, `normalize`, `parent`, `file_name`, `ext`
- `std.fs`: `exists`, `is_file`, `is_dir`, `read_text`, `write_text`, `list_dir`, `mkdir_all`, `remove_file`, `remove_dir_all`
- `std.fs` binary APIs: `read_bytes`, `write_bytes`
- `std.fs` stream APIs: `stream_open_read`, `stream_open_write`, `stream_read`, `stream_write`, `stream_eof`, `stream_close`
- `std.process`: `exec_status`
- `std.bytes`: UTF-8 bytes conversions and byte-array helpers (`Array[Int]` model)
- `std.text`: byte-oriented UTF-8 helpers (`len_bytes`, `byte_at`, `slice_bytes`, `starts_with`, `ends_with`, `split_lines`)
- Native runtime host-root sandbox enforcement for filesystem APIs.
- Native process execution capability gate (`--allow-process` required).
- VM deterministic unsupported diagnostics for host APIs.
- Host-tool MVP example at `examples/selfhost_host_tool_mvp`.
- Arcana frontend verification MVP at `examples/selfhost_frontend_mvp`.

## Excluded
- Network/socket APIs.
- Full Unicode grapheme/text segmentation APIs.
