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
- Bare-method qualifiers over concrete receiver methods must carry resolved callable identity through lowering.
- Impl methods on public receiver types are part of the public bare-method surface by default; cross-package bare-method use does not require a separate `export` marker on each impl method.
- When multiple runtime routines share the same callable path, lowered executable rows must also carry exact concrete routine identity for that bare-method call instead of leaving runtime to re-disambiguate from receiver shape.
- Trait-bound or otherwise generic bare-method calls may remain receiver-directed dynamic dispatch in the current runtime lane when lowering cannot know a concrete impl routine ahead of execution.
- Bare-method execution must route through linked routines or approved intrinsic seams rather than executor-owned public-std qualifier shims.

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
