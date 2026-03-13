# Qualified Phrases v1 Scope

Status: `approved-pre-selfhost`

This scope freezes the current qualified-phrase contract for the rewrite.

## Surface Model

- The general phrase family is `subject :: args? :: qualifier`.
- Qualified phrases remain a first-class Arcana call/operation surface, not a parsing convenience that runtime may reinterpret ad hoc.
- The current qualifier family includes:
  - `call`
  - bare method / path qualifiers
  - apply-style qualifier `>`
  - try-propagation qualifier `?`
  - await-apply qualifier `>>`
  - dotted callable paths such as `canvas.blit`

## Resolution Contract

- Qualified-phrase resolution is a frontend responsibility.
- Lowered executable artifacts must carry:
  - resolved callable identity for direct call subjects and dotted callable qualifiers
  - qualifier kind
  - call mode / access-mode shape
  - any attachment metadata that survives validation
- Runtime must execute the lowered result.
- Runtime must not reconstruct dotted-path identity from dynamic receiver shape as the source of truth.
- Bare-method qualifiers use receiver-type-directed method resolution in the current bootstrap runtime lane.
- That bare-method lookup must execute linked routines or approved intrinsic seams rather than executor-owned public-std qualifier shims.
- Native/backend hardening may replace the bootstrap lookup mechanism later without changing the public source surface.

## Dotted Qualifiers

- Dotted qualifiers are direct callable paths, not merely receiver-method sugar.
- If frontend resolves a dotted qualifier to a specific symbol path, that identity must survive lowering and execution.

## Try And Await-Apply

- `expr :: :: ?` remains part of the frozen language surface.
- `task_expr :: :: >>` remains part of the frozen language surface.
- These are explicit qualifier behaviors, not aliases for generic method lookup.

## Attachments

- Qualified-phrase attachments follow syntax-side qualifier rules.
- If validated metadata is retained through lowering, executor/runtime must tolerate its presence rather than failing simply because the phrase carried approved metadata.

## Phrase Arity

- The 3 top-level inline argument cap remains intentional.
- Larger inputs must be shaped explicitly as pair/record data rather than using phrase arity as a reason to add implicit callable/context features.
