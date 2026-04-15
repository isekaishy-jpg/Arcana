# Best-Fix Wave for Review Correctness Items, With `arcana_process` as the Stream Contract

## Summary

- Fix the concrete correctness findings in the review, using the strongest end-state fixes that still fit one implementation wave.
- Exclude findings `14`, `15`, `17`, and `18` from this wave:
  - `14`, `15`, and `17` belong to the broader `arcana_winapi` surface-removal/reduction work in `18`
  - `16` remains docs/source drift, not an implementation-correctness fix
- Keep `arcana_process.fs` as the public stream contract. The fix is to make the internal runtime/backend implementation conform to it, not to replace it with a runtime-only API.

## Public / Interface Changes

- `arcana-cabi`
  - raw-pointer decode/free helpers become `unsafe fn` or crate-private raw helpers behind validated safe wrappers
  - add one canonical validated decode/release helper layer used by runtime and generated support
- `RuntimeCoreHost`
  - add internal stream lifecycle methods so host implementations can uphold real `arcana_process.fs` stream semantics:
    - open read
    - open write
    - read
    - write
    - eof
    - close
  - these are backend hooks, not a new public Arcana source surface
- HIR/frontend internals
  - post-resolution semantic lookup must use `package_id`
  - visible package computation becomes package-id-based, not package-name-based
- Foreword adapter identity
  - introduce one canonical resolved-runner identity record:
    - manifest runner token
    - resolved executable path
    - executable digest
  - introduce one canonical sidecar manifest/digest set used by both staging and replay identity
- Package/build/distribution internals
  - replace weak bundle readiness with one shared material-identity validator reused by build-cache reuse and staged-bundle reuse

## Implementation Changes

### 1. Canonical CABI transport safety
- Move owned/view/layout decode and release semantics behind one validated helper layer in `arcana-cabi`.
- Remove safe unconstrained raw-pointer-to-reference or raw-pointer-to-owned-value helpers from normal public use.
- Make runtime binding input decode consume validated bytes/views from the canonical CABI helper layer instead of reinterpreting raw pointers locally.
- Harden native product descriptor reading through one audited helper:
  - reject `count > 0 && ptr == null`
  - bound counts before allocation/iteration
  - keep raw slice construction inside narrow `unsafe`
- Update generated support and runtime callback/import paths to call the canonical helper layer instead of re-stating transport semantics.

### 2. `arcana_process.fs` stream semantics, implemented correctly
- Keep `arcana_process.fs` exactly as the public contract frozen in the selfhost-host scope.
- Remove the runtime pathname-reopen `FileStream` implementation.
- Extend `RuntimeCoreHost` so the host owns real stream semantics across the full lifecycle.
- Change runtime `FileStream` execution to:
  - create a runtime-visible stream handle that maps to a host-owned open stream token
  - route `stream_read`, `stream_write`, `stream_eof`, and `stream_close` back through the host trait
  - never reopen by stored pathname during later operations
- Implement the new host trait methods in `BufferedHost` with real open file handles and host-owned state.
- Keep `arcana_winapi.helpers.process` out of semantic ownership; it is only one backend/provider path and must conform to the same `arcana_process.fs` semantics if/when it is used.

### 3. Singular resolved package identity
- Make `package_id` the only semantic identity after graph resolution across HIR lookup, frontend semantic checks, and trait/method visibility.
- Replace post-resolution uses of `workspace.package(name)` / `resolved.package(name)` in semantic code with `package_by_id(...)`.
- Replace visible-package-name computation with visible-package-id computation and use that in:
  - method candidate lookup
  - trait impl visibility
  - owner/object/module semantic checks
- Remove silent fallback from failed resolved-package lookup to the current module; resolved identity failure must hard-fail with a diagnostic/error.
- Keep name-based lookup only for source parsing, diagnostics, and clearly-marked test/display helpers.

### 4. Singular adapter execution identity
- Introduce one canonical adapter identity builder that produces:
  - resolved runner executable path
  - runner digest
  - same-stem sidecar manifest with sorted `(name, digest)` entries
  - main product digest
- Make both staged-root naming/materialization and in-process replay-cache identity consume that same canonical record.
- Resolve bare runner names through PATH/PATHEXT before both hashing and launching.
- Remove stale sidecars from staged roots when the source sidecar set shrinks.
- Use one identity model for:
  - staging root naming
  - replay-cache keys
  - execution-time validation

### 5. Singular build/bundle material identity
- Remove the `>64` support-file correctness downgrade.
- Make support-file identity material for every set:
  - normalized relative path
  - file size
  - file hash
  - stable aggregate digest
- Introduce one Windows-aware normalized distribution-path identity and use it in validation and copy planning so case-only collisions are rejected before writes.
- Replace `distribution_bundle_is_ready` with a material validator that checks:
  - root artifact identity
  - toolchain identity
  - support-file material identity
  - staged manifest identity
- Reuse that same validator for build-cache reuse and staged-bundle reuse.
- Centralize directory walking for package/build/distribution/AOT:
  - `symlink_metadata`
  - no recursion through symlink/junction/reparse-point directories by default
  - visited-directory tracking for cycle protection
- Tighten destructive reset behavior:
  - only clear matching Arcana-managed bundle dirs
  - refuse to clear non-empty mismatched `out_dir`

## Test Plan

- **CABI / runtime transport**
  - malformed descriptor tests for every descriptor family: null pointer + non-zero count must return `Err`
  - tests proving no safe raw-pointer borrow helper remains in normal use
  - CABI tests around validated decode/release wrappers and unsafe raw helper boundaries
- **`arcana_process.fs` runtime behavior**
  - open a stream, replace the path target on disk, then read/write/eof: the logical stream stays attached to the original open handle
  - sandboxed host tests proving later stream operations do not bypass host-controlled open semantics
  - `BufferedHost` coverage for open/read/write/eof/close through the new trait methods
- **Resolved package identity**
  - duplicate display-name transitive graph where only one branch exports a method/impl; only the resolved visible package id is honored
  - resolved package lookup failure hard-fails instead of silently using the current module
- **Foreword adapter identity**
  - bare runner name resolves to different PATH executable => identity changes
  - sidecar content change => identity changes
  - sidecar deletion => identity changes and stale sidecar is removed
  - replay cache and staged-root identity stay in lockstep
- **Build / distribution**
  - support set over 64 files: mutate one file and prove cache miss
  - parseable but stale/corrupted staged bundle: prove restage
  - Windows case-only support-file collision: prove failure before copy
  - symlink/junction cycle tests in traversed trees
  - non-empty mismatched `out_dir`: prove reset refuses to clear it

## Assumptions And Defaults

- `arcana_process.fs` remains the approved public semantic owner for filesystem stream behavior.
- `RuntimeCoreHost` is the internal implementation seam that must conform to `arcana_process.fs`; it is not a competing public contract.
- Findings `14`, `15`, `17`, and `18` are deferred to the separate `arcana_winapi` cleanup/removal plan and must not be accidentally strengthened here.
- Finding `16` stays out of scope here because it is docs/source drift, not a concrete implementation-correctness fix.
- Approved specs remain authoritative; this wave changes implementation and narrow internal/public API shape only where needed to match them.
