# Seed Import

This repository imports only the frozen contract and source corpus needed for the rewrite.

Imported from `MeadowLang`:
- `docs/arcana-v0.md`
- `docs/specs/chain`
- `docs/specs/forewords`
- `docs/specs/memory`
- `docs/specs/selfhost-host`
- `docs/specs/backend/ir-backend-roadmap.md`
- `docs/specs/backend/selfhost_language_contract_v1.md`
- `conformance/selfhost_language_matrix.toml`
- `conformance/check_parity_fixtures`
- `conformance/fixtures/types_guard_workspace`
- `std`
- `grimoires/arcana-frontend`
- `grimoires/arcana-compiler-core`
- `grimoires/arcana-selfhost-compiler`
- `grimoires/winspell`
- `grimoires/spell-events`
- `examples`

Intentionally excluded:
- legacy Rust implementation crates
- `PLAN*.md`
- `tmp/`
- copied `.arcana/` caches
- copied `Arcana.lock`, `API.lock`, and `CONSUMERS.lock`
- generated compile artifacts and golden-output bundles
- oversized generated direct-emit payload shards that exceed GitHub's hard file limit

The imported source corpus is treated as behavioral reference only. It does not grant permission to change the frozen Arcana language before selfhost.

Current omission:
- `grimoires/arcana-compiler-core/src/direct_emit_specs_061.arc` was replaced with a minimal placeholder module because the carried-over generated payload exceeded GitHub's 100 MB per-file limit. The original behavior must be regenerated later if the new toolchain needs that direct-emit snapshot during bootstrap work.
