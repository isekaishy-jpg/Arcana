# Objects And Owners v1 Scope

Status: `approved-pre-selfhost`

This file records the pre-selfhost Arcana surface for `obj`, `create ... scope-exit`, owner activation, availability attachments, and callable/context object roles.

## Goals

- Add explicit packaged state and managed lifetime domains before desktop/grimoire growth hardens.
- Keep lifetime, cleanup, and re-entry rules explicit and statically understandable.
- Keep dispatch static and compiler-auditable.
- Avoid closures, implicit capture, trait objects, and hidden activation.

## Non-Goals

- No closures or lambdas.
- No general function-value reopening.
- No trait-object or runtime-vtable callable dispatch.
- No implicit global state creation from imports or attachments.
- No separate stored-object keyword in v1.

## Core Forms

Object declaration:

```arc
obj Counter:
    value: Int
```

Owner declaration:

```arc
create Session [Counter] scope-exit:
    done: when Counter.value >= 10 retain [Counter]
```

Owner declaration with explicit activation context:

```arc
create Session [Counter] context: SessionCtx scope-exit:
    done: when Counter.value >= 10 retain [Counter]
```

Availability attachment:

```arc
Session
Counter
fn main() -> Int:
    let active = Session :: :: call
    Counter.value = 1
    return Counter.value
```

## Objects

- `obj` defines a nominal packaged unit of state with optional nested methods.
- Object fields use the same declaration form as record fields.
- Object methods are ordinary function-like declarations nested under the object body.
- Object lifecycle hooks use ordinary nested methods named `init` and `resume`.
- Allowed lifecycle hook forms are:
  - `fn init(edit self: Self):`
  - `fn init(edit self: Self, read ctx: Ctx):`
  - `fn resume(edit self: Self):`
  - `fn resume(edit self: Self, read ctx: Ctx):`
- Lifecycle hooks must return `Unit`, must not be async, and must not declare type parameters.
- Callable objects and context objects are roles of ordinary `obj` declarations, not separate declaration kinds.
- Existing `impl` and trait participation remains valid for object types.

## Owners

- `create Owner [ObjectA, ObjectB, ...] scope-exit:` declares a managed lifetime domain.
- Owners may optionally declare an explicit owner-level activation context with `context: Ctx` before `scope-exit:`.
- Each listed object is owned under that owner.
- An owner must declare at least one exit clause.
- Exit names must be unique within the owner.
- `retain [...]` may retain only objects declared on that owner.

Exit clause forms:

- `exit when condition`
- `name: when condition`
- either form may append `retain [ObjectA, ObjectB]`

## Availability

- Bare path lines immediately above a block-owning header attach availability, not live state.
- Availability attachments may target owners or objects only.
- Attachment never initializes state by itself.
- Attachment never creates free initialized locals by itself.
- Direct object-name access inside a block requires both attachment and active owner state on that execution path.

## Activation And Re-entry

- Owner entry and re-entry use qualified phrase call syntax on the owner symbol.
- The current v1 activation form is statement-form or `let`-binding owner activation:

```arc
let active = Session :: ctx :: call
Session :: ctx :: call
```

- Activation may carry zero or one context argument.
- Owners without a `context:` clause accept zero activation args.
- Owners with a `context:` clause require exactly one activation arg of that type on entry and re-entry.
- Activation introduces the owner handle into active scope.
- Attached owned-object names become directly usable locals while that owner is active in the attached scope.
- That active-owner state carries through ordinary routine calls and newly entered attached blocks on the same execution path; attached helpers do not require explicit re-entry when the caller already has the owner active.
- Re-attaching the object name is sufficient for direct object access on that path; helpers and nested blocks do not need to re-attach the owner when the same owner is already active.
- If an owner declares `context: Ctx`, owned object lifecycle hooks may either omit context or use that exact `Ctx` type.
- Mixed per-object lifecycle context types are not part of the current owner contract once an owner uses an explicit `context:` clause.

## Exit Checkpoints

Owner exits are evaluated at explicit checkpoints:

- owner entry or re-entry while prior owner state is still active
- successful mutation of active owner-backed state
- structured block or routine exit after local `defer` work and cleanup footer work complete

If an exit condition resolves true:

- when multiple exit conditions resolve true at the same checkpoint, the first matching exit in source order wins
- held owned objects remain in the owner domain for later re-entry
- non-held owned objects are cleaned up deterministically

## Suspend And Re-init

- In current v1 terms, suspension is modeled as owner exit plus `retain [...]`.
- Held objects remain packaged under the owner after exit, but they are not active again until explicit re-entry.
- Re-entry follows the same zero-arg vs one-arg rule as entry based on the owner's declared `context:` clause and may resume previously held state or realize fresh non-held objects on demand.
- Fresh realized owned objects run `init` once on first realization for the current activation.
- Held objects resumed into a new activation run `resume` once on first access for that activation.
- If only a context-taking lifecycle hook exists, that object requires activation with a matching context before the hook can run.
- Attachment alone does not resume suspended owner state.

## Cleanup Order

- Local `defer` work runs before cleanup footer work for the same header, per the existing approved cleanup contract.
- Owner exit cleanup runs after those local cleanup rings for the exiting scope.

## Static Dispatch And Explicitness

- Dispatch remains static only.
- Object/callable/context support must not introduce closure capture or dynamic callable transport.
- Import exposes definitions only.
- Availability exposes entry/use possibility only.
- Explicit activation is the only way owner-backed live state becomes active.
