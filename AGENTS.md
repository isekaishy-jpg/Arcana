# Arcana Repo Instructions

## Authority Order

Use this order when judging current Arcana behavior:

1. `POLICY.md`
2. approved/frozen specs under `docs/specs/`
3. `PLAN.md` and `docs/rewrite-roadmap.md`
4. `crates/*`
5. `llm.md`
6. active first-party source packages: `std/`, `grimoires/arcana/*`

## Current Repo Reality

- `std/` is first-party rewrite surface.
- `grimoires/arcana/*` is the core/tooling grimoire layer.
- no active `grimoires/libs/*` layer is present in the current workspace.
- `docs/reference/*`, `examples/`, and `conformance/` are supporting material, not primary architecture authority by themselves.

## Use `llm.md`

- `llm.md` is the quick guide for Arcana source form, crate lookup, and common parser/frontend/runtime gotchas.
- It helps with source work, but it does not outrank approved specs or `crates/*`.

## Review Rule

- Distinguish:
  - approved contract
  - crate implementation
  - active first-party source package shape
  - examples/fixtures/reference drift
- If source packages or examples conflict with approved docs and `crates/*`, treat that as drift or implementation debt, not silent contract change.
