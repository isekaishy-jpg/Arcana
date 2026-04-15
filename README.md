# Arcana

Arcana is a source-first language and tooling stack being rebuilt in Rust under a hard pre-selfhost language freeze.

This repository contains:
- the rewrite-era compiler and tooling crates
- the rewrite-owned `std` surface
- the rewrite-owned core grimoires under `grimoires/arcana`
- examples and conformance material used to pressure the new toolchain

## Status

What is working now:
- deterministic package and workspace planning with local path deps plus a local versioned registry source
- `arcana check`, `arcana build`, `arcana run`, and `arcana package`
- `arcana publish <workspace-dir> --member <member>` for machine-local versioned lib publication
- typed frontend and internal IR/runtime execution for the approved pre-selfhost language surface
- native Windows `exe` / `dll` packaging
- rewrite-owned core grimoires for WinAPI bindings plus host/process/audio substrate

What is not done yet:
- selfhost
- named remote registries and Git dependencies
- stable public backend artifact contracts
- first-party `arcana test`, `arcana format`, and `arcana review`
- any rebuilt higher-level app/media grimoires above the current substrate

The language surface is frozen until selfhost except for contract-preserving fixes and the already-approved owner/object exception.

## Quick Start

Prerequisites:
- Rust toolchain
- Windows if you want to exercise the native packaging path

Check the Rust workspace:

```powershell
cargo check --workspace
```

Build the Rust workspace:

```powershell
cargo build --workspace
```

Check a first-party package:

```powershell
cargo run -p arcana-cli --bin arcana -- check grimoires/arcana/winapi
```

Package a minimal native bundle:

```powershell
cargo test -q -p arcana-cli --features windows-native-bundle-tests package_workspace_stages_runnable_windows_exe_bundle -- --nocapture
```

## CLI Commands

The rewrite CLI currently supports:

```text
arcana check <path>
arcana build <workspace-dir> [--plan] [--target <target>]
arcana run <workspace-dir> [--target <target>] [--member <member>] [-- <args...>]
arcana package <workspace-dir> [--target <target>] [--member <member>] [--out-dir <dir>]
arcana publish <workspace-dir> --member <member>
```

Supported build targets today:
- `internal-aot`
- `windows-exe`
- `windows-dll`

Dependency source support today:
- local path dependencies
- local published versioned dependencies from the built-in `local` registry source

## Repository Layout

- [crates](crates): Rust rewrite implementation for syntax, HIR, frontend, IR, package/build, runtime, AOT, and CLI
- [std](std): rewrite-owned first-party standard library surface
- [grimoires/arcana](grimoires/arcana): rewrite-owned core grimoires such as WinAPI bindings and process/host-core
- [examples](examples): checked-in proof workspaces and fixtures
- [docs/specs](docs/specs): frozen and approved rewrite contracts
- [conformance](conformance): explicit language and contract fixtures

## Read First

If you are trying to understand or contribute to the rewrite, start here:
- [POLICY.md](POLICY.md)
- [AGENTS.md](AGENTS.md)
- [docs/specs/spec-status.md](docs/specs/spec-status.md)
- [PLAN.md](PLAN.md)
- [docs/rewrite-roadmap.md](docs/rewrite-roadmap.md)
- [llm.md](llm.md) for Arcana source-form guidance and crate lookup notes
- [docs/specs/os-bindings/os-bindings/v1-scope.md](docs/specs/os-bindings/os-bindings/v1-scope.md)
- [docs/specs/selfhost-host/selfhost-host/v1-scope.md](docs/specs/selfhost-host/selfhost-host/v1-scope.md)

Current rewrite authority order:
1. `POLICY.md`
2. approved and frozen specs under `docs/specs/`
3. `PLAN.md` and `docs/rewrite-roadmap.md`
4. `crates/*`

## Current Boundaries

- no pre-selfhost language expansion
- no remote registry or Git dependencies yet
- no stable public backend artifact contract yet
- `std` is rewrite-owned first-party surface, not archived MeadowLang architecture to preserve wholesale
- core grimoires sit above the substrate and are meant to own their public contracts rather than act as thin wrappers over third-party crates
- `arcana_process` is the public host-core/process owner; old public `std.*` host-core lanes are retired

## License

[LICENSE](LICENSE)
