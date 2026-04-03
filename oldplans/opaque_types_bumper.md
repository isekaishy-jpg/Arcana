# Opaque Type Declarations: One-Shot Pre-Selfhost Add

## Summary

Add a new source-level `opaque type` declaration so runtime/resource handles stop living as hardcoded Rust-only builtin names.

Chosen defaults:
- Scope: `trusted-only` for now, enforced to package `std`
- Syntax: `opaque type`
- Policy: inline and required
- First migration target: `runtime handles only`, not concurrency/memory handles yet

Primary syntax:
```arc
export opaque type Window as move, boundary_unsafe
export opaque type AudioPlayback as move, boundary_unsafe
```

This is a general language feature in shape, but pre-selfhost use is restricted to `std` so the runtime handle model can move into source now without reopening the wider language surface.

## Key Changes

### 1. Add a real opaque type declaration to syntax/HIR/frontend
- Add top-level declaration parsing for `opaque type <Name>[type_params?] as <policy atoms>`.
- Required policy atoms in v1:
  - one ownership atom: `copy` or `move`
  - one boundary atom: `boundary_safe` or `boundary_unsafe`
- Disallow bodies on opaque declarations in v1.
- Allow normal `export` visibility.
- Allow impl targets and trait impl targets to reference opaque types exactly like other named types.
- Treat opaque types as type-like for:
  - type-surface resolution
  - API fingerprinting
  - impl target resolution
  - trait/method lookup
- Do not allow opaque types to behave like records/enums in expressions:
  - no constructor phrase resolution
  - no field access assumptions
  - no payload/variant semantics

Implementation shape:
- `crates/arcana-syntax`: add `SymbolKind::OpaqueType` plus parsed opaque policy struct
- `crates/arcana-hir`: mirror opaque symbol kind and policy
- `crates/arcana-frontend`: resolve opaque types as named types, read ownership/boundary behavior from the resolved declaration, and reject opaque constructor use

### 2. Enforce trusted-only pre-selfhost use
- In the frontend, reject `opaque type` declarations outside package `std`.
- Do not special-case module paths beyond that in v1; package-level trust is enough.
- `std.kernel.*` may use opaque types in signatures, but declarations themselves live in the public owning `std.*` modules, not in `std.kernel.*`.

### 3. Migrate current runtime handles out of the Rust builtin registry
Migrate only these now:
- `Window`
- `Image`
- `FileStream`
- `AudioDevice`
- `AudioBuffer`
- `AudioPlayback`
- `AppFrame`

Owning declaration locations:
- `std.window` owns `Window`
- `std.canvas` owns `Image`
- `std.fs` owns `FileStream`
- `std.audio` owns `AudioDevice`, `AudioBuffer`, `AudioPlayback`
- `std.events` owns `AppFrame`

Then:
- update active `std` modules and `std.kernel.*` modules to import and use those source-declared types explicitly
- update owned grimoires and the still-checked reference app grimoires to follow the new imports
- remove those seven names from the Rust builtin type registry
- keep the remaining builtin registry only for the still-unmigrated core/concurrency/memory families

### 4. Keep the compiler coupling narrow and future-proof
- Keep a centralized Rust builtin registry, but shrink it to the non-migrated families only.
- Make ownership inference and boundary checks prefer resolved opaque declarations over builtin-name lookup.
- Keep the new feature general enough that later migration of `Task`, `Thread`, `Channel`, `Mutex`, atomics, and memory arenas/ids uses the same mechanism without another syntax redesign.
- Do not migrate those additional families in this pass.

### 5. Update docs and contract wording
- Update `docs/arcana-v0.md` to define `opaque type` and its v1 restrictions.
- Update `docs/specs/std/std/v1-scope.md` and `docs/specs/std/std/v1-status.md` to say runtime handles are now source-declared opaque std types, not Rust-only reserved names.
- Update `docs/specs/selfhost-host/selfhost-host/app-substrate-v1-scope.md` to describe the migrated runtime handles as std-owned opaque declarations.
- Update `docs/rewrite-roadmap.md` only if it still frames runtime handles as compiler-only bootstrap names.

## Test Plan

### Syntax / HIR
- Parse/lower valid examples:
  - `export opaque type Window as move, boundary_unsafe`
  - `opaque type Token[T] as move, boundary_safe`
- Reject:
  - missing `as` clause
  - missing ownership atom
  - missing boundary atom
  - duplicate/conflicting atoms
  - body attached to opaque type
  - unsupported policy atoms

### Frontend semantics
- Reject `opaque type` outside package `std`.
- Allow opaque types in type positions, impl targets, and trait impl targets.
- Reject opaque-type constructor use in expressions.
- Ownership flow:
  - use-after-close / use-after-stop still rejected through resolved opaque declarations, not builtin-name fallback
- Boundary checks:
  - `boundary_unsafe` opaque types rejected for boundary signatures
  - `boundary_safe` opaque types accepted

### Migration / integration
- `arcana check std`
- `cargo test -p arcana-syntax -p arcana-hir -p arcana-frontend -p arcana-package -p arcana-cli`
- `arcana check grimoires\owned\app\arcana-desktop`
- `arcana check grimoires\owned\app\arcana-audio`
- `arcana check grimoires\reference\app\winspell`
- `arcana check grimoires\reference\app\spell-events`
- `arcana check grimoires\reference\app\spell-audio`

### Regression guard
- Add a Rust-side test asserting the runtime handle names are no longer present in the builtin registry after migration.
- Add frontend tests asserting resolved opaque declarations, not builtin-name tables, drive ownership and boundary behavior for the migrated families.

## Assumptions

- This pass is not the full removal of builtin types from the compiler; it is the runtime-handle migration pass.
- Concurrency and memory handle families stay builtin for now because migrating them is not required to unblock the current roadmap.
- `opaque type` is non-constructible in v1 and exists to model handles/resources, not user-instantiable hidden records.
- Generic opaque types are allowed by syntax design so the feature does not need another redesign later, even though the first migration set is non-generic.
