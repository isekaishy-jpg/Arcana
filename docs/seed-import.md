# Seed Import

This repository imports only the frozen contract and source corpus needed for the rewrite.

Imported from `MeadowLang`:
- `docs/arcana-v0.md`
- `docs/specs/chain`
- `docs/specs/forewords`
- `docs/specs/memory`
- `docs/specs/selfhost-host`
- `docs/specs/backend/selfhost_language_contract_v1.md`
- `conformance/selfhost_language_matrix.toml`
- `conformance/check_parity_fixtures`
- `conformance/fixtures/types_guard_workspace`
- `std`
- `grimoires/reference/toolchain/arcana-frontend`
- `grimoires/reference/toolchain/arcana-compiler-core`
- `grimoires/reference/toolchain/arcana-selfhost-compiler`
- `grimoires/reference/app/winspell`
- `grimoires/reference/app/spell-events`
- `grimoires/reference/examples` (imported from the original top-level `examples/` tree)

Imported as historical context only and now superseded/reference-only:
- `docs/reference/backend/ir-backend-roadmap.md`
- `docs/reference/forewords/generic-foreword-spec.md`
- `docs/reference/memory/v2-scope.md`
- `docs/reference/selfhost-host/generic-host-spec.md`

Intentionally excluded:
- legacy Rust implementation crates
- `PLAN*.md`
- `tmp/`
- copied `.arcana/` caches
- copied `Arcana.lock`, `API.lock`, and `CONSUMERS.lock`
- generated compile artifacts and golden-output bundles
- oversized generated direct-emit payload shards that exceed GitHub's hard file limit

The imported source corpus is treated as behavioral reference only. It does not grant permission to change the frozen Arcana language before selfhost.
Imported planning material does not define rewrite architecture unless `docs/specs/spec-status.md` classifies it as current authority.
Imported `std` and `grimoires/reference/*` are behavioral carryover only: their current layering, helper inventory, runtime assumptions, and backend couplings are not rewrite authority.
Rebuild `std` around the rewrite architecture and move showcase/game-specific logic back out into showcase/app grimoires where appropriate.
Carried `winspell` and `spell-events` express the requirement for first-party window/input/canvas and primitive graphics/text support, not a commitment to MeadowLang's prior `winit`/VM/bytecode implementation stack.
The current `grimoires/` tree is a mixed migration workspace, not a promise that Arcana's final bootstrap/selfhost package layout matches the carried MeadowLang split.
Track carried std modules and future Arcana-owned app/media grimoire roles through `docs/specs/std/std/v1-status.md` and `docs/specs/grimoires/grimoires/v1-status.md` so imported behavior does not silently become rewrite authority.
Architecture/selfhost-progress review should privilege approved docs plus `crates/*`; imported `std`, `grimoires/reference/*`, and generated direct-emit artifacts are migration corpus unless a current scope explicitly ratifies the exact surface under discussion.
Reference corpus is expected to shrink over time rather than remain a permanent parallel development tree. Once the rewrite-owned runtime/app substrate is working, owned grimoires are usable, and an owned showcase exists, `grimoires/reference/*` should move out of normal default validation and remain only as selective migration/conformance pressure until final archive or deletion.

Current omission:
- `grimoires/reference/toolchain/arcana-compiler-core/src/direct_emit_specs_061.arc` was replaced with a minimal placeholder module because the carried-over generated payload exceeded GitHub's 100 MB per-file limit. The original behavior must be regenerated later if the new toolchain needs that direct-emit snapshot during bootstrap work.
