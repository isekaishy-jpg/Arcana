# Arcana

Arcana is a source-first language and tooling stack being rebuilt in Rust under a hard pre-selfhost language freeze.

This repository contains:
- the rewrite-era compiler and tooling crates
- the rewrite-owned `std` surface
- the rewrite-owned core grimoires under `grimoires/arcana`
- the rewrite-owned app/media grimoires under `grimoires/libs`
- examples and conformance material used to pressure the new toolchain

The current native path is Windows-first and already packages a real desktop proof app through `arcana_desktop` and `arcana_graphics.arcsb`.

## Status

What is working now:
- deterministic package and workspace planning with local path deps plus a local versioned registry source
- `arcana check`, `arcana build`, `arcana run`, and `arcana package`
- `arcana publish <workspace-dir> --member <member>` for machine-local versioned lib publication
- typed frontend and internal IR/runtime execution for the approved pre-selfhost language surface
- native Windows `exe` / `dll` packaging
- rewrite-owned core grimoires for WinAPI bindings and host/process substrate
- rewrite-owned desktop and graphics backend grimoires sufficient to drive the checked-in desktop proof workspace

What is not done yet:
- selfhost
- named remote registries and Git dependencies
- stable public backend artifact contracts
- first-party `arcana test`, `arcana format`, and `arcana review`
- full `arcana_text` closure on the current rewritten surface

The language surface is frozen until selfhost except for contract-preserving fixes and the already-approved owner/object exception.

## Quick Start

Prerequisites:
- Rust toolchain
- Windows if you want to package and run the native desktop showcase

Check the Rust workspace:

```powershell
cargo check --workspace
```

Build the Rust workspace:

```powershell
cargo build --workspace
```

Check a workspace:

```powershell
cargo run -p arcana-cli -- check examples/arcana-desktop-proof
```

Package the desktop showcase:

```powershell
cargo run -p arcana-cli -- package examples/arcana-desktop-proof --target windows-exe --member arcsb_app
```

Run the desktop proof directly:

```powershell
cargo run -p arcana-cli -- run examples/arcana-desktop-proof --target windows-exe --member arcsb_app
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

## Desktop Showcase

The checked-in showcase workspace lives at [examples/arcana-desktop-proof](examples/arcana-desktop-proof).

The active checked-in member is `arcsb_app`.

It demonstrates the current owned desktop shell and software-buffer backend through the public grimoire boundary:
- window creation and redraw flow through `arcana_desktop`
- mapped software-buffer presentation through `arcana_graphics.arcsb`
- resize, buffer-age, and present-with-damage behavior
- packaging through the real Windows native bundle path

This is the current proof workspace for the owned desktop + graphics backend lane, not a substrate-only mock.

## Repository Layout

- [crates](crates): Rust rewrite implementation for syntax, HIR, frontend, IR, package/build, runtime, AOT, and CLI
- [std](std): rewrite-owned first-party standard library surface
- [grimoires/arcana](grimoires/arcana): rewrite-owned core grimoires such as WinAPI bindings and process/host-core
- [grimoires/libs](grimoires/libs): rewrite-owned app/media grimoires such as desktop, graphics, text, and audio
- [examples](examples): checked-in proof workspaces, including the desktop showcase
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
- [docs/specs/grimoires/grimoires/v1-scope.md](docs/specs/grimoires/grimoires/v1-scope.md)
- [docs/specs/selfhost-host/selfhost-host/app-substrate-v1-scope.md](docs/specs/selfhost-host/selfhost-host/app-substrate-v1-scope.md)

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
- owned grimoires sit above the substrate and are meant to own their public contracts rather than act as thin wrappers over third-party crates
- `arcana_process` is the public host-core/process owner; old public `std.*` host-core lanes are retired

## License

[LICENSE](LICENSE)
