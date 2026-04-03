# Qualified Phrases v1 Scope

Status: `approved-pre-selfhost`

This scope freezes the current qualified-phrase contract for the rewrite.

## Surface Model

- The general phrase family is `subject :: args? :: qualifier`.
- Qualified phrases remain a first-class Arcana call/operation surface, not a parsing convenience that runtime may reinterpret ad hoc.
- The current qualifier family includes:
  - `call`
  - bare method qualifiers
  - named path qualifiers
  - apply-style qualifier `>`
  - try-propagation qualifier `?`
  - await-apply qualifier `>>`
  - await qualifier `await`
  - spawn qualifiers `weave` and `split`
  - failure qualifiers `must` and `fallback`
- `call`, bare-method, and named-path qualifiers may carry explicit qualifier type args such as:
  - `subject :: args :: call[T]`
  - `subject :: args :: method[T]`
  - `subject :: args :: pkg.fn[T]`

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

## Await, Spawn, And Failure Qualifiers

- `subject :: :: await` is the phrase-native await form.
- `callable_subject :: args :: weave` is the phrase-native task-spawn form.
- `callable_subject :: args :: split` is the phrase-native thread-spawn form.
- `subject :: :: must` is the phrase-native hard unwrap for `Option[T]` and `Result[T, Str]`.
- `subject :: fallback_value :: fallback` is the phrase-native fallback form for `Option[T]` and `Result[T, Str]`.
- `must` accepts zero args.
- `fallback` accepts exactly one positional fallback arg and no named args.
- `split` remains conservative about cross-thread `edit` place capture until a broader transferable-place law is approved.

## Attachments

- Qualified-phrase attachments follow syntax-side qualifier rules.
- Named attachment entries are currently allowed only for `call`, bare-method, and `>`.
- Named attachment entries are rejected for named-path, `?`, `>>`, `await`, `weave`, `split`, `must`, and `fallback`.
- If validated metadata is retained through lowering, executor/runtime must tolerate its presence rather than failing simply because the phrase carried approved metadata.

## Phrase Arity

- The 3 top-level inline argument cap remains intentional.
- Larger inputs must be shaped explicitly as pair/record data rather than using phrase arity as a reason to add implicit callable/context features.
