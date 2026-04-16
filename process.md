# `arcana_process` Ownership And Process-Lane Consolidation

## Summary

- Make `arcana_process` the only public and runtime-known host-core owner for `args`, `env`, `io`, `path`, `fs`, and `process`.
- Keep the internal semantics seam in the runtime layer we already hardened. `arcana_process` is the privileged sys grimoire; runtime should stay keyed to public `arcana_process.*` names, not a shadow `sys_*` namespace.
- Move `arcana_process.io` onto that same runtime seam so it no longer inherits behavior through `std.kernel.io`.
- Clean up the process-related WinAPI surface now so there is only one semantic lane: `arcana_winapi` keeps canonical handle types and backend glue, but not a peer public behavior API for host-core process/path/fs/env/args/io.

## Key Changes

### 1. `arcana_process` is the canonical public and runtime-known surface
- Keep the approved public API and signatures unchanged:
  - `arcana_process.args`
  - `arcana_process.env`
  - `arcana_process.io`
  - `arcana_process.path`
  - `arcana_process.fs`
  - `arcana_process.process`
- Keep runtime direct-call ownership on those public `arcana_process.*` routine keys.
- Allow private `sys_*` modules under `grimoires/arcana/process/src/` only as source-side factoring helpers if useful, but they are not runtime-known keys and not semantic owners.
- Public modules stop forwarding to:
  - `arcana_winapi.helpers.process`
  - `std.kernel.io`
- Public modules keep Arcana-owned shaping where appropriate:
  - `env.get_or`
  - `print_line` / `eprint_line`
  - `ExecCapture` record construction and helper methods
  - `Cleanup[FileStream]`

### 2. Runtime seam owns semantics for all six modules
- The existing runtime host-core seam remains the internal semantic owner for:
  - args/env access
  - stdout/stderr/read_line
  - cwd and path operations
  - filesystem operations
  - `FileStream` lifecycle
  - process execution
- `arcana_process.io` moves onto that same seam; it is no longer implemented through `std.kernel.io`.
- Keep the corrected `FileStream` behavior from the earlier correctness work:
  - host-backed open handles
  - no later pathname reopen
- Keep public `arcana_process.*` behavior and runtime semantics aligned by construction:
  - one public routine name
  - one runtime implementation owner
  - optional private source helpers only underneath

### 3. Uniform host-core policy
- `arcana_process.fs` must always inherit the runtime sandbox/path policy:
  - host-root-constrained resolution
  - real-path containment
  - rejection of `..` escape, absolute-path escape, and symlink/junction escape
  - same policy for read, write, list, copy, rename, remove, and stream open
- `arcana_process.process.exec_status` and `exec_capture` must always inherit the runtime process policy:
  - explicit allow-process gate
  - one executable-resolution policy
  - no alternate public lane calling `Command::new(...)` directly
- Keep lexical path helpers in the same semantic owner rather than splitting them back into WinAPI wrappers.

### 4. WinAPI process cleanup now
- Remove `arcana_winapi.helpers.process` from the public `arcana_winapi.helpers` reexport surface.
- Stop treating `grimoires/arcana/winapi/src/helpers/process.arc` as a public Arcana-facing behavior layer.
- Keep `arcana_winapi.process_handles.FileStream` public and canonical.
- Internalize or delete process-related semantic wrappers in WinAPI:
  - no public `arg_count`, `env_get`, path/fs/process execution helper lane
  - no public helper-layer result reconstruction / `take_last_error` protocol for host-core behavior
- Keep only substrate/backend glue that is still needed under the runtime boundary.
- Update docs/spec-adjacent wording so `arcana_process` is plainly the privileged sys host-core layer going forward, while `arcana_winapi` is substrate plus canonical handle declarations.

## Test Plan

- Ownership tests:
  - public `arcana_process.{args,env,io,path,fs,process}` lowers and runs without importing `arcana_winapi.helpers.process` or `std.kernel.io`
  - runtime direct-call table remains keyed to public `arcana_process.*`
- Filesystem policy tests on the public surface:
  - `..` escape rejected
  - absolute-path escape rejected
  - symlink/junction escape rejected
  - destructive ops obey the same policy
  - stream open/read/write/eof/close still match the corrected handle-backed semantics
- Process policy tests on the public surface:
  - `exec_status` denied when process execution is disabled
  - `exec_capture` denied when process execution is disabled
  - allow-path happy case still works
  - executable resolution uses the same policy regardless of backend path
- Boundary tests:
  - no public `grimoires/arcana/process/src/*.arc` module imports `arcana_winapi.helpers.process`
  - `grimoires/arcana/process/src/io.arc` no longer imports `std.kernel.io`
  - `arcana_winapi.helpers.process` is not publicly reexported
- End-to-end checks:
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace --quiet`

## Assumptions

- Scope is all six `arcana_process` modules, not just `fs` and `process`.
- `arcana_process` is the privileged sys host-core layer going forward; that direction is decided, not tentative.
- The internal host-core seam stays in runtime for this wave; no new public or runtime-known `std.kernel.{args,env,path,fs,process}` family is introduced.
- `std.kernel.io` may remain only as unrelated std/runtime machinery if still needed elsewhere, but it is no longer the implementation substrate for `arcana_process.io`.
- `arcana_winapi` remains the canonical owner of `process_handles.FileStream`, but not of public host-core behavior.
