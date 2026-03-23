# Arcana

Arcana is a source-first language and tooling stack being rebuilt in Rust under a hard pre-selfhost language freeze.

This repository contains:
- the rewrite-era compiler and tooling crates
- the rewrite-owned `std` surface
- the rewrite-owned app/media grimoires
- examples and conformance material used to pressure the new toolchain

The current native path is Windows-first and already packages a real desktop showcase through the owned desktop stack.

## Status

What is working now:
- deterministic path-only package and workspace planning
- `arcana check`, `arcana build`, `arcana run`, and `arcana package`
- typed frontend and internal IR/runtime execution for the approved pre-selfhost language surface
- native Windows `exe` / `dll` packaging
- rewrite-owned desktop, graphics, and text grimoires sufficient to drive the checked-in desktop showcase

What is not done yet:
- selfhost
- Git or registry dependencies
- stable public backend artifact contracts
- first-party `arcana test`, `arcana format`, and `arcana review`

The language surface is frozen until selfhost except for contract-preserving fixes and the already-approved owner/object exception.

## Quick Start

Prerequisites:
- Rust toolchain
- Windows if you want to package and run the native desktop showcase

Run the workspace tests:

```powershell
cargo test --workspace
```

Check a workspace:

```powershell
cargo run -p arcana-cli -- check examples/arcana-desktop-proof
```

Package the desktop showcase:

```powershell
cargo run -p arcana-cli -- package examples/arcana-desktop-proof --target windows-exe --member app
```

Launch the packaged app:

```powershell
examples\arcana-desktop-proof\dist\app\windows-exe\app.exe
```

## CLI Commands

The rewrite CLI currently supports:

```text
arcana check <path>
arcana build <workspace-dir> [--plan] [--target <target>]
arcana run <workspace-dir> [--target <target>] [--member <member>] [-- <args...>]
arcana package <workspace-dir> [--target <target>] [--member <member>] [--out-dir <dir>]
```

Supported build targets today:
- `internal-aot`
- `windows-exe`
- `windows-dll`

Dependency source support today:
- local path dependencies only

## Desktop Showcase

The checked-in showcase workspace lives at [examples/arcana-desktop-proof](examples/arcana-desktop-proof).

It demonstrates the current owned desktop shell through the public grimoire boundary:
- multi-window session control
- window state/configuration and whole-record apply paths
- cursor, text-input, IME, clipboard, monitor, and wake flows
- raw device events and device-event policy
- packaging through the real Windows native bundle path

This is the current proof workspace for the owned desktop grimoire, not a substrate-only mock.

## Repository Layout

- [crates](crates): Rust rewrite implementation for syntax, HIR, frontend, IR, package/build, runtime, AOT, and CLI
- [std](std): rewrite-owned first-party standard library surface
- [grimoires/owned](grimoires/owned): rewrite-owned app/media grimoires such as desktop, graphics, text, and audio
- [examples](examples): checked-in proof workspaces, including the desktop showcase
- [docs/specs](docs/specs): frozen and approved rewrite contracts
- [conformance](conformance): explicit language and contract fixtures

## Read First

If you are trying to understand or contribute to the rewrite, start here:
- [POLICY.md](POLICY.md)
- [docs/specs/spec-status.md](docs/specs/spec-status.md)
- [PLAN.md](PLAN.md)
- [docs/rewrite-roadmap.md](docs/rewrite-roadmap.md)
- [docs/specs/grimoires/grimoires/v1-scope.md](docs/specs/grimoires/grimoires/v1-scope.md)
- [docs/specs/selfhost-host/selfhost-host/app-substrate-v1-scope.md](docs/specs/selfhost-host/selfhost-host/app-substrate-v1-scope.md)

Current rewrite authority order:
1. `POLICY.md`
2. approved and frozen specs under `docs/specs/`
3. `PLAN.md` and `docs/rewrite-roadmap.md`
4. `crates/*`

## Current Boundaries

- no pre-selfhost language expansion
- no Git or registry dependencies yet
- no stable public backend artifact contract yet
- `std` is rewrite-owned first-party surface, not archived MeadowLang architecture to preserve wholesale
- owned grimoires sit above the substrate and are meant to own their public contracts rather than act as thin wrappers over third-party crates

## License

[LICENSE](LICENSE)
