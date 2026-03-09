# Chain Contract Matrix v1

## Scope
- Plan: `PLAN46.md`
- Date: 2026-03-03
- Applies to chain contracts on stage declarations (`#stage[...]`) and chain statements (`#chain[...]`).

## Surface Model
- Chain surface is decomposed into:
  - style qualifier (`forward`, `lazy`, `parallel`, `async`, `plan`, `broadcast`, `collect`)
  - introducer family (`:=>` for forward/mixed, `:=<` for composition)
  - connector-directed stage edges (`=>`, `<=`)
- Style selects execution semantics and capability rules.
- Introducer selects the chain family.
- Connectors express per-edge flow inside that family.

## Required System-Boundary Resolution
For chain phrases inside `behavior[...]`/`system[...]` bodies, resolution must provide:
- `phase`
- `thread`
- `authority`
- `deterministic`
- aggregate `reads`/`writes`/`excludes` sets

Resolution source:
- explicit `#chain[...]` values
- aggregated stage contracts (`#stage[...]`)
- boundary defaults (`phase`/`thread` from behavior/system declaration, `authority=local`)

## Simulation Matrix
Typical fixed/update simulation pipelines:

| Property | Fixed Tick | Update Tick |
|---|---|---|
| `phase` | `fixed` | `update` |
| `deterministic` | `true` (required) | `true` preferred |
| `thread` | `worker` or `main` | `worker` or `main` |
| `authority` | `local`/`server` | `local`/`server` |
| `effect` | `read`/`write`/`exclusive_write` | `read`/`write` |

## ECS Matrix
Resource-access contracts used by scheduler grouping:

| Access shape | Parallel eligibility |
|---|---|
| `reads(A)` + `reads(A)` | allowed |
| `writes(A)` + `reads(A)` | conflict |
| `writes(A)` + `writes(A)` | conflict |
| `excludes(A)` + any access to `A` | not grouped together |

Notes:
- compiler assigns deterministic scheduler groups (source-order first-fit) per phase.
- conflicting access patterns are separated into different groups instead of requiring compile-time rejection.
- source order tie-break applies inside equal-priority groups.

## Networking Matrix
Authority-oriented chains:

| Domain | Recommended `authority` | Typical `phase` |
|---|---|---|
| local prediction | `client` or `local` | `update` |
| authoritative simulation | `server` | `fixed`/`net` |
| decode/ingest | `server`/`client` | `net` |

Rules:
- chain/stage authority must be compatible with enclosing boundary authority.
- incompatible merges emit authority-violation diagnostics.

## Render/Update Matrix
Thread and effect constraints:

| Phase | Thread | Effects |
|---|---|---|
| `render` | `main` | `render`, `read`, `emit` |
| `update` | `worker`/`main` | `read`, `write`, `emit` |

Rules:
- `thread=main` stage cannot be merged into a worker-only chain.
- deterministic claims should avoid non-deterministic effect paths.

## Recommended First-Party Defaults
For pure transform stages:
- `#stage[pure=true, deterministic=true, thread=any, authority=local, rollback_safe=true, effect=read]`

For behavior/system chains:
- `#chain[phase=<boundary-phase>, deterministic=true, thread=<boundary-thread>, authority=local, rollback_safe=true]`
