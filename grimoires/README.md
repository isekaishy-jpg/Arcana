# Grimoires Workspace

This directory holds both rewrite-owned grimoire scaffolds and reference/imported corpus.

Rules:
- Packages under `grimoires/owned/*` are rewrite-owned app/media grimoire scaffolds.
- Packages under `grimoires/reference/*` are reference/imported corpus only.
- Reference packages may still be checked, compared, or used by migration examples, but that does not make their layout authoritative.
- Current authority for future Arcana-owned app/media grimoire roles is `docs/specs/grimoires/grimoires/v1-scope.md` and `docs/specs/grimoires/grimoires/v1-status.md`.
- Those docs freeze required responsibilities and substrate boundaries, not the package split of the carried reference corpus.

Current contents:
- `owned/app/*`
- `reference/toolchain/*`
- `reference/app/*`
