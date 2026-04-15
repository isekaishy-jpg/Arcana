# Seed Import

This repository imports only the frozen contract and source corpus needed for the rewrite.

Originally seed-imported from `MeadowLang`:
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
- broad reference toolchain/app/example corpus, now archived outside this repo

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

The imported source corpus was bootstrap context only. It did not grant permission to change the frozen Arcana language before selfhost.
Imported planning material does not define rewrite architecture unless `docs/specs/spec-status.md` classifies it as current authority.
`std` has been rebuilt as rewrite-owned first-party surface; historical MeadowLang toolchain/app corpus is archived outside the repo and is not rewrite authority.
Historical `winspell`/`spell-events` style corpus expressed the requirement for first-party window/input/canvas and primitive graphics/text support, not a commitment to MeadowLang's prior VM/bytecode implementation stack.
The current `grimoires/` tree is rewrite-owned, not a promise that Arcana's final bootstrap/selfhost package layout matches Meadow-era splits.
Track rewrite-owned `std` modules through `docs/specs/std/std/v1-status.md`.
Architecture/selfhost-progress review should privilege approved docs plus `crates/*`; archived historical corpus and generated direct-emit artifacts are migration context unless a current scope explicitly ratifies the exact surface under discussion.
