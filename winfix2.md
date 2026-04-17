# `arcana_winapi` Raw-Only Cleanup and `arcana_process.fs.FileStream` Host-Core Handle Shift

## Summary

- Reduce public `arcana_winapi` to a `windows-sys`-style surface: `arcana_winapi.raw.*` only.
- Remove the remaining public helper/wrapper/handle layer from `arcana_winapi` in one hard break. No compatibility aliases.
- Treat `FileStream` as a real host-core ownership change, not a rename:
  - `arcana_process.fs.FileStream` becomes the only public stream-handle type
  - `arcana_process.fs` owns the full public stream contract and `Cleanup`
  - `arcana_winapi.process_handles.FileStream` is deleted
- Keep all Windows/native/CABI stream representation private behind runtime/backend seams so no public API implies stream handles are binding-owned Win32 handles.

## Public / Interface Changes

- `arcana_winapi` public exports become `raw.*` only. `backend/*` stays internal and is never reexported.
- Delete these public `arcana_winapi` modules entirely:
  - `helpers`
  - `foundation`
  - `fonts`
  - `windows`
  - `types`
  - `desktop_handles`
  - `graphics_handles`
  - `audio_handles`
  - `process_handles`
- In `arcana_process.fs`:
  - define `type FileStream: move`
  - `stream_open_read` / `stream_open_write` return `Result[FileStream, Str]`
  - `stream_read(edit stream: FileStream, ...)`
  - `stream_write(edit stream: FileStream, ...)`
  - `stream_eof(read stream: FileStream, ...)`
  - `stream_close(take stream: FileStream) -> Result[Unit, Str]`
  - `impl Cleanup[FileStream] for FileStream`
- Do not leave any public alias, wrapper, or doc wording that preserves `arcana_winapi.process_handles.FileStream` as a valid path.

## Implementation Changes

### 1. Raw-only public `winapi`
- Change `grimoires/arcana/winapi/src/book.arc` so public reexports are limited to `arcana_winapi.raw`.
- Delete the public wrapper/compat surface instead of leaving empty shells.
- Keep surviving Win32 glue only under internal backend/shackle files. Rename internal files if needed so the remaining implementation reads as backend glue, not public helpers.

### 2. Move `FileStream` into `arcana_process` as a host-core model change
- Remove `use arcana_winapi.process_handles.FileStream` from `grimoires/arcana/process/src/fs.arc` and define the type there directly.
- Update every public `arcana_process.fs` routine and cleanup impl to use the process-owned type path.
- Delete `grimoires/arcana/winapi/src/process_handles.arc`.
- Remove any runtime/frontend/AOT/package assumptions that the canonical stream-handle path lives under `arcana_winapi`.
- Runtime host-core remains the semantic owner of stream validity and behavior:
  - stream open/read/write/eof/close stay routed through the host seam
  - the runtime owns the public handle model and lifetime rules
  - any OS handle, file descriptor, or CABI carrier remains backend-private

### 3. Remove the remaining non-process public substrate
- Delete the current public window/message/text-input/clipboard/graphics/text/audio surface from `arcana_winapi`.
- Remove the associated non-process public handle families in the same wave.
- If some code is still needed for internal callback translation or native bootstrap, keep it backend-private only. Do not preserve any public API for it.

### 4. Update consumers, tests, and docs to the new boundary
- Update runtime embedded source lists, package-shape tests, frontend fixtures, and AOT/runtime module assumptions so they no longer reference removed `arcana_winapi` public modules.
- Update approved docs in the same change:
  - `docs/specs/os-bindings/os-bindings/v1-scope.md`: public `arcana_winapi` becomes raw-only
  - `docs/specs/resources/resources/v1-scope.md`: remove `arcana_winapi` desktop/graphics/audio/process handle families and replace stream-handle references with `arcana_process.fs.FileStream`
  - `docs/specs/selfhost-host/selfhost-host/v1-scope.md`: canonical stream-handle path becomes `arcana_process.fs.FileStream`
- Update active roadmap/reference docs so `winapi` is described as raw bindings plus internal backend glue only.

## Test Plan

- Public boundary tests:
  - `arcana_winapi` publicly exports only `raw.*`
  - no public `helpers`, `foundation`, `fonts`, `windows`, `types`, or `*_handles` modules remain
- Stream-handle ownership tests:
  - `arcana_process.fs.FileStream` parses, lowers, typechecks, runs, and cleans up correctly
  - no repo consumer references `arcana_winapi.process_handles.FileStream`
  - stream behavior still follows the runtime host-core contract, including valid/invalid handle behavior and consuming close
- Removal tests:
  - no repo consumer imports removed `arcana_winapi` helper/wrapper modules
  - no repo consumer references removed non-process handle paths
  - no repo consumer depends on `arcana_winapi` for typed handles or helper routines anymore; remaining imports, if any, are `arcana_winapi.raw.*` only
- Full validation:
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace --quiet`

## Assumptions

- This is a hard public break by design. No compatibility aliases or migration wrapper lane is kept.
- `arcana_process` remains the semantic owner of host-core file/process behavior; this wave finishes the ownership shift by moving the stream handle type there too.
- Non-process public `winapi` capability is intentionally removed even though no replacement higher-level owner is introduced in the same wave.
- Any remaining Windows-specific file/process/message/native machinery is allowed only as backend-private implementation under `arcana_winapi`, never as public API.
